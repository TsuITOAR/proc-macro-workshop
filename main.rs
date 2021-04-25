// Write code here.
//
// To see what the code looks like after macro expansion:
//     $ cargo expand
//
// To run the code:
//     $ cargo run
use derive_builder::Builder;

#[derive(Builder)]
#[allow(dead_code)]
pub struct Command {
	#[builder(each="s")]
    executable: String,
    args: Vec<String>,
    env: Vec<String>,
    current_dir: Option<String>,
}
fn main() {
    Command {
        executable: "1".to_string(),
        args: vec!["1".to_string()],
        env: vec!["1".to_string()],
        current_dir: None,
    }.current_dir.is_none();
}
