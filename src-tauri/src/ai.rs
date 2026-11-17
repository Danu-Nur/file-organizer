// SPDX-License-Identifier: GPL-3.0-or-later
//! Local AI in-process with llama.cpp (text/JSON only).
//! One worker thread owns the model = the queue; the model is released after IDLE; GPU (Vulkan) first, CPU fallback;
//! JSON through llama.cpp's common sampler with a grammar; qwen3.5 thinking switched off with an empty think block.
//! Document text is read into memory for one question and never stored or logged.
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Mutex, OnceLock};
use std::time::Duration;

use llama_cpp_4::context::params::LlamaContextParams;
use llama_cpp_4::llama_backend::LlamaBackend;
use llama_cpp_4::llama_batch::LlamaBatch;
use llama_cpp_4::model::params::LlamaModelParams;
use llama_cpp_4::model::{AddBos, LlamaChatMessage, LlamaModel, Special};

const IDLE: Duration = Duration::from_secs(300);
const BATCH: usize = 512;

struct Job {
    model: PathBuf,
    system: String,
    user: String,
    schema: serde_json::Value,
    max_tokens: usize,
    reply: mpsc::Sender<Result<String, String>>,
}

fn ask(model: &Path, system: &str, user: String, schema: serde_json::Value, max_tokens: usize) -> Result<serde_json::Value, String> {
    let (reply, rx) = mpsc::channel();
    send(Job { model: model.into(), system: system.into(), user, schema, max_tokens, reply })?;
    let out = rx.recv().map_err(|_| "The local AI worker stopped.".to_string())??;
    serde_json::from_str(&out).map_err(|e| e.to_string())
}

/// Pick one of `folders` for a document (or "-" for none). Returns (folder, short reason).
pub fn pick_folder(model: &Path, name: &str, path: &Path, folders: &[String]) -> Result<(String, String), String> {
    choose(model, name, &read_text(path).unwrap_or_default(), folders)
}

pub fn choose(model: &Path, name: &str, text: &str, folders: &[String]) -> Result<(String, String), String> {
    let mut options = folders.to_vec();
    options.push("-".into());
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "folder": { "type": "string", "enum": options }, "alasan": { "type": "string", "maxLength": 120 } },
        "required": ["folder", "alasan"]
    });
    let system = "Kamu membantu merapikan file. Pilih SATU subfolder yang paling cocok untuk dokumen ini berdasarkan nama dan isinya. \
                  Jika tidak ada yang cocok, pilih \"-\". Tulis alasan singkat dalam bahasa Indonesia.";
    let body = if text.is_empty() { "(isi tidak terbaca — nilai dari nama saja)".to_string() } else { text.to_string() };
    let v = ask(model, system, format!("Nama file: {name}\nSubfolder: {}\n\nIsi awal dokumen:\n{body}", folders.join(", ")), schema, 200)?;
    Ok((v["folder"].as_str().unwrap_or("-").into(), v["alasan"].as_str().unwrap_or("").into()))
}

/// Propose up to 8 short topic folder names for a set of documents (name + opening text), reusing `existing` ones.
pub fn propose_topics(model: &Path, docs: &[(String, String)], existing: &[String]) -> Result<Vec<String>, String> {
    let schema = serde_json::json!({
        "type": "object",
        "properties": { "topics": { "type": "array", "minItems": 1, "maxItems": 8, "items": { "type": "string", "maxLength": 40 } } },
        "required": ["topics"]
    });
    let system = "Kamu membantu merapikan dokumen. Buat daftar 2-8 nama folder topik yang singkat (1-3 kata, bahasa Indonesia, \
                  boleh nama perusahaan/proyek) untuk mengelompokkan dokumen berikut. Pakai ulang folder yang sudah ada bila cocok. \
                  Jangan membuat folder untuk satu dokumen saja kecuali tidak ada pilihan lain.";
    let mut list = String::new();
    for (i, (name, text)) in docs.iter().enumerate() {
        let snippet: String = text.chars().take(160).collect();
        list.push_str(&format!("{}. {name} — {snippet}\n", i + 1));
    }
    let v = ask(model, system, format!("Folder yang sudah ada: {}\n\nDokumen:\n{list}", if existing.is_empty() { "(belum ada)".into() } else { existing.join(", ") }), schema, 300)?;
    let mut topics: Vec<String> = v["topics"].as_array().into_iter().flatten()
        .filter_map(|t| t.as_str().map(|s| s.trim().trim_end_matches('.').to_string()))
        .filter(|t| !t.is_empty() && crate::ops::valid_part(t).is_ok())
        .collect();
    topics.dedup();
    Ok(topics)
}

/// First ~2000 characters of a PDF / DOCX / text file.
pub fn read_text(p: &Path) -> Option<String> {
    let ext = p.extension()?.to_str()?.to_lowercase();
    let text = match ext.as_str() {
        "pdf" => {
            let p = p.to_path_buf();
            std::panic::catch_unwind(move || pdf_extract::extract_text(&p)).ok()?.ok()?
        }
        "docx" => {
            let mut z = zip::ZipArchive::new(std::fs::File::open(p).ok()?).ok()?;
            let mut xml = String::new();
            std::io::Read::read_to_string(&mut z.by_name("word/document.xml").ok()?, &mut xml).ok()?;
            let mut out = String::new();
            let mut in_tag = false;
            for c in xml.chars() {
                match c {
                    '<' => { in_tag = true; out.push(' '); }
                    '>' => in_tag = false,
                    _ if !in_tag => out.push(c),
                    _ => {}
                }
            }
            out
        }
        "txt" | "md" | "rtf" => {
            let b = std::fs::read(p).ok()?;
            String::from_utf8_lossy(&b[..b.len().min(8000)]).into_owned()
        }
        _ => return None,
    };
    let t: String = text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(2000).collect();
    (!t.is_empty()).then_some(t)
}

fn send(job: Job) -> Result<(), String> {
    static TX: OnceLock<Mutex<mpsc::Sender<Job>>> = OnceLock::new();
    let tx = TX.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::Builder::new().name("local-ai".into()).spawn(move || worker(rx)).expect("spawn local AI worker");
        Mutex::new(tx)
    });
    tx.lock().unwrap().send(job).map_err(|_| "The local AI worker is not running.".to_string())
}

fn worker(rx: mpsc::Receiver<Job>) {
    let backend = match LlamaBackend::init() {
        Ok(b) => b,
        Err(e) => {
            for job in rx {
                job.reply.send(Err(format!("Local AI could not start: {e}"))).ok();
            }
            return;
        }
    };
    let mut loaded: Option<(PathBuf, LlamaModel)> = None;
    loop {
        let job = match rx.recv_timeout(IDLE) {
            Ok(j) => j,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                loaded = None; // give RAM/VRAM back
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => return,
        };
        // A panic inside the llama.cpp bindings must not kill the worker (every later job would hang).
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let model = ensure_model(&backend, &mut loaded, &job.model)?;
            run_json(&backend, model, &job.system, &job.user, &job.schema, job.max_tokens)
        }))
        .unwrap_or_else(|_| {
            loaded = None;
            Err("The local AI failed on this request.".to_string())
        });
        job.reply.send(result).ok();
    }
}

fn ensure_model<'a>(backend: &LlamaBackend, loaded: &'a mut Option<(PathBuf, LlamaModel)>, path: &Path) -> Result<&'a LlamaModel, String> {
    if loaded.as_ref().map(|l| l.0.as_path()) != Some(path) {
        *loaded = None;
        if !path.exists() {
            return Err(format!("File model AI tidak ditemukan: {}", path.display()));
        }
        // Loading needs ~2 GB (weights + context); refuse rather than push the machine into swapping.
        let (_, avail) = crate::sysinfo::ram();
        if avail > 0 && avail < 2 * (1 << 30) {
            return Err(format!("RAM tersedia hanya {} — AI tidak dijalankan. Tutup aplikasi lain lalu coba lagi.", crate::sysinfo::gb(avail)));
        }
        let m = match LlamaModel::load_from_file(backend, path, &LlamaModelParams::default().with_n_gpu_layers(99)) {
            Ok(m) => m,
            Err(_) => LlamaModel::load_from_file(backend, path, &LlamaModelParams::default().with_n_gpu_layers(0))
                .map_err(|e| format!("Model AI tidak bisa dimuat: {e}"))?,
        };
        *loaded = Some((path.into(), m));
    }
    Ok(&loaded.as_ref().unwrap().1)
}

fn context_params(n_ctx: usize) -> LlamaContextParams {
    let threads = std::thread::available_parallelism().map(|n| (n.get() / 2).max(2) as i32).unwrap_or(4);
    LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx as u32))
        .with_n_batch(BATCH as u32)
        .with_n_threads(threads)
        .with_n_threads_batch(threads)
}

fn run_json(backend: &LlamaBackend, model: &LlamaModel, system: &str, user: &str, schema: &serde_json::Value, max_tokens: usize) -> Result<String, String> {
    use llama_cpp_4::common_sampler::{CommonSampler, CommonSamplerParams, GrammarSource};
    let gbnf = llama_cpp_4::chat::json_schema_to_grammar(&schema.to_string(), false).map_err(|e| format!("Invalid answer schema: {e}"))?;
    let msgs = vec![
        LlamaChatMessage::new("system".into(), system.into()).map_err(|e| e.to_string())?,
        LlamaChatMessage::new("user".into(), user.into()).map_err(|e| e.to_string())?,
    ];
    let mut prompt = model.apply_chat_template(None, &msgs, true).map_err(|e| format!("Chat template failed: {e}"))?;
    if model.chat_template(None).map(|t| t.contains("<think>")).unwrap_or(false) {
        prompt.push_str("<think>\n\n</think>\n\n");
    }
    let toks = model.str_to_token_with_special(&prompt, AddBos::Always, Special::Tokenize).map_err(|e| e.to_string())?;
    let n_ctx = (toks.len() + max_tokens + 64).next_multiple_of(1024).clamp(4096, 32768);
    let mut ctx = model.new_context(backend, context_params(n_ctx)).map_err(|e| format!("Local AI context failed: {e}"))?;
    let keep = toks.len().min(n_ctx.saturating_sub(max_tokens + 64));
    let toks = &toks[..keep];
    let mut batch = LlamaBatch::new(BATCH, 1);
    for (c, chunk) in toks.chunks(BATCH).enumerate() {
        batch.clear();
        for (j, t) in chunk.iter().enumerate() {
            let i = c * BATCH + j;
            batch.add(*t, i as i32, &[0], i == toks.len() - 1).map_err(|e| e.to_string())?;
        }
        ctx.decode(&mut batch).map_err(|e| format!("Local AI read failed: {e}"))?;
    }

    let mut params = CommonSamplerParams::new();
    let mut sc = params.scalars();
    sc.temp = 0.0;
    params.set_scalars(&sc);
    params.set_grammar(&gbnf, GrammarSource::User, false).map_err(|e| e.to_string())?;
    let mut sampler = CommonSampler::new(model, &mut params).map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    let (mut depth, mut in_str, mut esc) = (0i32, false, false);
    let mut n_past = toks.len() as i32;
    let mut idx = batch.n_tokens() - 1;
    for _ in 0..max_tokens {
        let tok = sampler.sample(&mut ctx, idx, false).map_err(|e| format!("Local AI sampling failed: {e}"))?;
        sampler.accept(tok, true);
        if model.is_eog_token(tok) {
            break;
        }
        let piece = model.token_to_bytes(tok, Special::Tokenize).unwrap_or_default();
        out.extend(&piece);
        let mut done = false;
        for &b in &piece {
            if esc { esc = false; continue; }
            match b {
                b'\\' if in_str => esc = true,
                b'"' => in_str = !in_str,
                b'{' if !in_str => depth += 1,
                b'}' if !in_str => { depth -= 1; if depth == 0 { done = true; } }
                _ => {}
            }
        }
        if done {
            break;
        }
        batch.clear();
        batch.add(tok, n_past, &[0], true).map_err(|e| e.to_string())?;
        n_past += 1;
        ctx.decode(&mut batch).map_err(|e| format!("Local AI write failed: {e}"))?;
        idx = 0;
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}
