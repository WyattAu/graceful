#[cfg(feature = "shutdown-flag")]
mod flag_tests {
    use shutdown_kit::flag::ShutdownFlag;

    #[test]
    fn new_flag_is_not_set() {
        let flag = ShutdownFlag::new();
        assert!(!flag.is_set());
    }

    #[test]
    fn set_makes_flag_set() {
        let flag = ShutdownFlag::new();
        flag.set();
        assert!(flag.is_set());
    }

    #[test]
    fn reset_clears_flag() {
        let flag = ShutdownFlag::new();
        flag.set();
        flag.reset();
        assert!(!flag.is_set());
    }

    #[test]
    fn clones_share_state() {
        let flag = ShutdownFlag::new();
        let flag2 = flag.clone();
        flag.set();
        assert!(flag2.is_set());
    }

    #[tokio::test]
    async fn wait_returns_immediately_when_set() {
        let flag = ShutdownFlag::new();
        flag.set();
        flag.wait().await;
    }

    #[tokio::test]
    async fn wait_blocks_until_set() {
        let flag = ShutdownFlag::new();
        let flag2 = flag.clone();
        tokio::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
            flag2.set();
        });
        flag.wait().await;
    }

    #[test]
    fn debug_shows_state() {
        let flag = ShutdownFlag::new();
        assert!(format!("{:?}", flag).contains("is_set: false"));
        flag.set();
        assert!(format!("{:?}", flag).contains("is_set: true"));
    }
}
