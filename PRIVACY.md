# Privacy

File Organizer is designed to keep everything on your computer.

## What is stored

In `%LOCALAPPDATA%\file-organizer\index.db` (SQLite):

- for each indexed file: name, full path, size, modification time and MD5 checksum;
- your decisions (dismissed suggestions), the undo log of moves, and settings.

File **contents are never stored**. Interface preferences (view mode, column widths) are kept in the app's local web
storage.

## What is read

- File contents are read to compute MD5 checksums (and discarded).
- PHP files and images are briefly scanned for suspicious code when you analyse a folder.
- With the optional AI switched on, the first ~2000 characters of a document are read into memory and given to the
  local model; nothing is saved or sent.

## Network

The app makes **no network requests**, with one exception: downloading the optional AI model from Hugging Face, and
only after you click "Unduh". No telemetry, no analytics, no accounts.

## Removing all data

Close the app and delete `%LOCALAPPDATA%\file-organizer\` (index, settings, downloaded model).
