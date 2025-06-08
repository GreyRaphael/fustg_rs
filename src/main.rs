use ctrlc;

mod engine;
mod strategy;
mod strategies;
mod types;
mod config;
mod operator;
mod broker;

use engine::CtaEngine;
use strategies::Aberration;
use types::SymbolType;

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

    // Add some strategies
    engine.add_strategy(SymbolType::from("rb2505"), Box::new(Aberration::new(100)));
    engine.add_strategy(SymbolType::from("rb2505"), Box::new(Aberration::new(200)));
    engine.add_strategy(SymbolType::from("MA505"), Box::new(Aberration::new(300)));
    engine.add_strategy(SymbolType::from("MA505"), Box::new(Aberration::new(400)));

    // Initialize worker threads, then enter the receive loop.
    engine.init();
    engine.start();

    // Once start() returns (because running was set to false), call stop()
    engine.stop();

    println!("Engine has shut down. Exiting main().");
}
