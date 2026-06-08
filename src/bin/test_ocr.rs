use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn send_and_read(stdin: &mut impl Write, stdout: &mut impl BufRead, msg: &str) -> String {
    writeln!(stdin, "{}", msg).unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    stdout.read_line(&mut line).unwrap();
    line.trim().to_string()
}

fn main() {
    let mut child = Command::new("target/debug/media-mcp.exe")
        .arg("--config")
        .arg("config.json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start server");

    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());

    let init = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}"#;
    let resp1 = send_and_read(&mut stdin, &mut stdout, init);
    println!("INIT: {}", &resp1[..200.min(resp1.len())]);

    // Test 1: default (auto) — OCR with confidence check
    let call1 = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_media","arguments":{"file_path":"D:\\workspace\\RustProjects\\media-mcp\\ScreenShot_2026-06-05_164142_386.png","language":"chi_sim+eng"}}}"#;
    let resp1 = send_and_read(&mut stdin, &mut stdout, call1);
    println!("=== AUTO mode ===");
    if resp1.contains("\"ai_description\"") {
        println!("-> Vision used (OCR confidence was below threshold)");
    } else {
        println!("-> OCR only (confidence was high enough)");
    }
    println!("Snippet: {}", &resp1[..1000.min(resp1.len())]);
    println!();

    // Test 2: vision="always" — force Vision
    let call2 = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_media","arguments":{"file_path":"D:\\workspace\\RustProjects\\media-mcp\\ScreenShot_2026-06-05_164142_386.png","language":"chi_sim+eng","vision":"always"}}}"#;
    let resp2 = send_and_read(&mut stdin, &mut stdout, call2);
    println!("=== ALWAYS mode ===");
    println!("Has ai_description: {}", resp2.contains("\"ai_description\""));
    println!("Snippet: {}", &resp2[..1000.min(resp2.len())]);
    println!();

    // Test 3: vision="skip" — OCR only, no Vision
    let call3 = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"read_media","arguments":{"file_path":"D:\\workspace\\RustProjects\\media-mcp\\ScreenShot_2026-06-05_164142_386.png","language":"chi_sim+eng","vision":"skip"}}}"#;
    let resp3 = send_and_read(&mut stdin, &mut stdout, call3);
    println!("=== SKIP mode ===");
    println!("Has ai_description: {}", resp3.contains("\"ai_description\""));
    println!("Full response length: {}", resp3.len());

    child.wait().unwrap();
}
