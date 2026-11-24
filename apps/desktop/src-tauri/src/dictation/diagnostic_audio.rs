//! Local diagnostic build: retain empty-recognition audio outside history pruning.
use std::path::{Path, PathBuf};

pub fn retain_no_speech(wav: &Path, directory: &Path) -> std::io::Result<PathBuf> {
    let name = wav.file_name().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing WAV name"))?;
    copy_recording(wav, directory).or_else(|_| {
        // A full/unwritable diagnostic destination must not leave the only copy as an orphan in audio/.
        // Its own subdirectory is ignored by both prune_orphan_audio and clear; rename requires no audio copy.
        let parent = wav.parent().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing WAV parent"))?;
        let fallback = parent.join("no-speech-diagnostics");
        std::fs::create_dir_all(&fallback)?;
        let saved = fallback.join(name);
        if saved.exists() {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "diagnostic already exists"));
        }
        std::fs::rename(wav, &saved)?;
        Ok(saved)
    })
}

fn copy_recording(wav: &Path, directory: &Path) -> std::io::Result<PathBuf> {
    let name = wav.file_name().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "missing WAV name"))?;
    let mut source = std::fs::File::open(wav)?;
    std::fs::create_dir_all(directory)?;
    let saved = directory.join(name);
    // Never overwrite an earlier diagnostic or delete the source if copying fails.
    let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&saved)?;
    std::io::copy(&mut source, &mut output)?;
    Ok(saved)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_recognition_keeps_exact_audio_outside_history() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("take.wav");
        let bytes = b"RIFF\x28\x00\x00\x00WAVEfmt \x10\x00\x00\x00\x01\x00\x01\x00\x80\x3e\x00\x00\x00\x7d\x00\x00\x02\x00\x10\x00data\x04\x00\x00\x00\x00\x00\xff\x7f";
        std::fs::write(&source, bytes).unwrap();
        let saved = retain_no_speech(&source, &root.path().join("diagnostics/no-speech")).unwrap();
        assert_eq!(std::fs::read(saved).unwrap(), bytes);
        assert_eq!(std::fs::read(source).unwrap(), bytes);
    }

    #[test]
    fn failed_diagnostic_copy_moves_audio_out_of_pruners_reach() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("take.wav");
        std::fs::write(&source, b"audio").unwrap();
        let blocked = root.path().join("not-a-directory");
        std::fs::write(&blocked, b"file").unwrap();
        let saved = retain_no_speech(&source, &blocked).unwrap();
        assert_eq!(saved, root.path().join("no-speech-diagnostics/take.wav"));
        assert_eq!(std::fs::read(saved).unwrap(), b"audio");
    }

    #[test]
    fn failure_of_both_destinations_leaves_original_intact() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("take.wav");
        std::fs::write(&source, b"audio").unwrap();
        let blocked = root.path().join("not-a-directory");
        std::fs::write(&blocked, b"file").unwrap();
        std::fs::write(root.path().join("no-speech-diagnostics"), b"file").unwrap();
        assert!(retain_no_speech(&source, &blocked).is_err());
        assert_eq!(std::fs::read(source).unwrap(), b"audio");
    }
}
