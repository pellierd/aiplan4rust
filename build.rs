use lalrpop::Configuration;

fn main() {
    let input_dir = "src/aiplan4rust/compiler/syntax/";
    let output_dir = "src/aiplan4rust/compiler/syntax/";

    std::fs::create_dir_all(output_dir).expect("Failed to create output directory");

    let mut config = Configuration::new();
    config.set_in_dir(input_dir);
    config.set_out_dir(output_dir);
    config.always_use_colors();
    //config.emit_comments(true);
    config.emit_report(true);
    config.emit_rerun_directives(true);
    config.log_verbose();
    config.log_debug();
    //    #[cfg(feature = "universal-preconditions")]
    config.process().unwrap();
}
