use std::path::PathBuf;

/// Process fallback for shells that do not report OSC 7. Run on the existing
/// background detector tick, only after output; never from layout or rendering.
#[derive(Default)]
pub(super) struct CwdObserver {
    content_seq: u64,
    cwd: Option<PathBuf>,
}

impl CwdObserver {
    pub(super) fn observe(
        &mut self,
        content_seq: u64,
        has_reported_cwd: bool,
        read_cwd: impl FnOnce() -> Option<PathBuf>,
    ) -> Option<PathBuf> {
        if has_reported_cwd || content_seq == self.content_seq || !content_seq.is_multiple_of(2) {
            return None;
        }
        self.content_seq = content_seq;
        let cwd = read_cwd().filter(|cwd| cwd.is_absolute())?;
        if self.cwd.as_ref() == Some(&cwd) {
            return None;
        }
        self.cwd = Some(cwd.clone());
        Some(cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quiet_in_flight_and_reported_output_never_probe_processes() {
        let mut observer = CwdObserver::default();
        for (seq, reported) in [(0, false), (1, false), (2, true)] {
            assert_eq!(
                observer.observe(seq, reported, || panic!("unexpected cwd probe")),
                None
            );
        }
    }

    #[test]
    fn output_changes_probe_once_and_only_changed_directories_are_published() {
        let first = std::env::temp_dir().join("first");
        let second = std::env::temp_dir().join("second");
        let mut observer = CwdObserver::default();
        assert_eq!(
            observer.observe(2, false, || Some(first.clone())),
            Some(first.clone())
        );
        assert_eq!(
            observer.observe(2, false, || panic!("quiet pane must not probe")),
            None
        );
        assert_eq!(observer.observe(4, false, || Some(first)), None);
        assert_eq!(
            observer.observe(6, false, || Some(second.clone())),
            Some(second)
        );
    }

    #[test]
    fn missing_and_relative_process_directories_are_ignored() {
        let mut observer = CwdObserver::default();
        assert_eq!(observer.observe(2, false, || None), None);
        assert_eq!(observer.observe(4, false, || Some("relative".into())), None);
    }
}
