use si_probe::catalog::*;
use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
};

fn serve_once(body: &'static str) -> String {
    let l = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}/catalog.json", l.local_addr().unwrap());
    thread::spawn(move || {
        let (mut s, _) = l.accept().unwrap();
        let mut buf = [0u8; 1024];
        let _ = s.read(&mut buf);
        let _ = write!(
            s,
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
    });
    url
}

#[test]
fn remote_catalog_used_when_valid() {
    let url = serve_once(r#"{"schema":1,"updated":"2030-01-01","entries":[]}"#);
    let (c, src) = load_catalog(&url);
    assert_eq!(
        (src, c.updated.as_str()),
        (CatalogSource::Remote, "2030-01-01")
    );
}
#[test]
fn wrong_schema_falls_back() {
    let url = serve_once(r#"{"schema":2,"updated":"2030-01-01","entries":[]}"#);
    assert_eq!(load_catalog(&url).1, CatalogSource::Bundled);
}
#[test]
fn offline_falls_back_to_bundled() {
    assert_eq!(
        load_catalog("http://127.0.0.1:9/none.json").1,
        CatalogSource::Bundled
    );
}
#[test]
#[ignore = "slow; needs network and Windows Update"]
fn wua_search_runs() {
    assert!(si_probe::search_windows_update_drivers().is_ok());
}
