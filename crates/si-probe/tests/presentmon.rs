use si_core::frames::FrameWindow;
use si_probe::presentmon::*;

#[test]
fn args_match_spec() {
    assert_eq!(
        presentmon_args("game.exe"),
        [
            "--process_name",
            "game.exe",
            "--output_stdout",
            "--no_console_stats",
            "--stop_existing_session",
            "--session_name",
            "SpeedImprover",
            "--terminate_on_proc_exit"
        ]
    );
}

#[test]
fn streams_frame_times_from_child() {
    #[cfg(unix)]
    let cmd = {
        let mut c = std::process::Command::new("sh");
        c.args([
            "-c",
            "printf 'Application,MsBetweenPresents\\ngame.exe,10.0\\ngame.exe,20.0\\n'",
        ]);
        c
    };
    #[cfg(windows)]
    let cmd = {
        let mut c = std::process::Command::new("cmd");
        c.args([
            "/C",
            "echo Application,MsBetweenPresents& echo game.exe,10.0& echo game.exe,20.0",
        ]);
        c
    };
    let s = PresentMonSession::start_command(cmd).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
    let mut w = FrameWindow::new(5000.0);
    assert_eq!(s.drain_into(&mut w), 2);
    assert_eq!(w.as_slice(), [10.0, 20.0]);
}
