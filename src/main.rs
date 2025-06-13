use ctrlc;

mod config;
mod engine;
mod operator;
mod perf_tracker;
mod strategies;
mod strategy;
mod types;

use engine::CtaEngine;
use strategies::Aberration;
use types::SymbolType;

use config::load_fees;
use perf_tracker::PerformanceTracker;

fn main() {
    // Register a Ctrl-C handler that just flips `running` to false.
    {
        ctrlc::set_handler(move || {
            println!("trigger Ctrl-C");
        })
        .expect("Error setting Ctrl-C handler");
    }

    // Build the engine, passing in the shared flag
    let mut engine = CtaEngine::new("ipc://@hq", "ipc://@orders", 4);

    let mut contracts = load_fees("config/fees.1st.toml").expect("load fees toml success");

    // Add some strategies
    if let Some(contract) = contracts.remove("SHFE.rb") {
        engine.add_strategy(
            SymbolType::from("rb2505"),
            Box::new(Aberration::new(100)),
            PerformanceTracker::new(1e6, contract),
        );
    }
    if let Some(contract) = contracts.remove("CZCE.MA") {
        engine.add_strategy(
            SymbolType::from("MA505"),
            Box::new(Aberration::new(200)),
            PerformanceTracker::new(1e6, contract),
        );
    }
    if let Some(contract) = contracts.remove("CZCE.MA") {
        engine.add_strategy(
            SymbolType::from("MA505"),
            Box::new(Aberration::new(300)),
            PerformanceTracker::new(1e6, contract),
        );
    }

    // Initialize worker threads, then enter the receive loop.
    engine.init();
    engine.start();

    // Once start() returns (because running was set to false), call stop()
    engine.stop();

    println!("Engine has shut down. Exiting main().");
}
