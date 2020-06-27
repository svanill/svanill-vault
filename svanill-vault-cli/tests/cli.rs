use assert_cmd::Command;

#[test]
fn it_output_version() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();

    let assert = cmd.args(&["-V"]).assert();

    assert.success().stdout("svanill-vault-cli 0.1.0\n");
}
