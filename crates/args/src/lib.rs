use log::info;
use std::sync::LazyLock;

pub static ENV_ARG: LazyLock<String> = LazyLock::new(|| {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let env = args[1].clone();
        info!("env arg: {}", env);
        return env;
    }
    "".to_string()
});

pub fn init_env_arg() {
    let _ = ENV_ARG.clone();
}
