//! Real loopback HTTP report, gated by files outside the saved run for deterministic tests.
use std::{
    io::{self, Read, Write},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

pub fn run(
    mode: &str,
    mut args: impl Iterator<Item = String>,
) -> Result<(), Box<dyn std::error::Error>> {
    if args.next().as_deref() != Some("--frame-ancestor") {
        return Err("frame ancestor required".into());
    }
    let ancestor = args.next().ok_or("ancestor missing")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    tokio::runtime::Runtime::new()?.block_on(async {
        if mode == "server-exit" { eprintln!("Synthetic server exit"); std::process::exit(17); }
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = format!("http://{}/report", listener.local_addr()?);
        let root = std::env::current_dir()?;
        let (closed, mut eof) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || { let _ = io::stdin().read_to_end(&mut Vec::new()); let _ = closed.send(()); });
        let mut announced = false;
        let mut extra = false;
        loop {
            if !announced && mode != "server-timeout" && (mode != "server-delayed" || root.join("render-release").exists()) {
                std::fs::write(root.join("render-address"), &address)?;
                println!("{}", if mode == "server-bad" { "http://other.invalid:80/report" } else { &address });
                io::stdout().flush()?;
                announced = true;
            }
            if mode == "server-extra" && !extra && root.join("render-extra").exists() {
                println!("unexpected extra output"); io::stdout().flush()?; extra = true;
            }
            tokio::select! {
                _ = &mut eof => return Ok(()),
                accepted = listener.accept() => {
                    let (mut stream, _) = accepted?;
                    let target = std::fs::read_to_string(root.join("report-target"))
                        .unwrap_or_else(|_| "http://127.0.0.1:9/blocked".into());
                    let body = if mode == "server-probe" { super::probe::html(&ancestor, &target) }
                        else { "<!doctype html><title>Sample report</title><h1>Saved report</h1>".into() };
                    let redirect = format!("HTTP/1.1 302 Found\r\nLocation: {target}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
                    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Security-Policy: frame-ancestors {ancestor}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
                    tokio::spawn(async move {
                        let mut request = [0u8;4096];
                        if matches!(tokio::time::timeout(Duration::from_secs(1), stream.read(&mut request)).await, Ok(Ok(n)) if n > 0) {
                            let response = if request.starts_with(b"GET /redirect ") { &redirect } else { &response };
                            let _ = stream.write_all(response.as_bytes()).await;
                            let _ = stream.shutdown().await;
                        }
                    });
                },
                _ = tokio::time::sleep(Duration::from_millis(10)) => (),
            }
        }
    })
}
