fn main() {
    if std::env::var("CARGO_FEATURE_PROFILING").is_ok() {
        sp1_build::build_program_with_args(
            "../program",
            sp1_build::BuildArgs {
                features: vec!["profiling".to_string()],
                ..Default::default()
            },
        );
    } else {
        sp1_build::build_program("../program");
    }
}
