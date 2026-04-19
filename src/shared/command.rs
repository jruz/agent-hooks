use std::process::Output;

pub fn combine_output(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    format!("{stderr}\n{stdout}").trim().to_string()
}
