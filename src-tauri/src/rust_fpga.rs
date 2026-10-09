use rust_hdl::prelude::*;

// Define a module with a parameter
#[derive(Clone, Debug, Default)]
pub struct ParamModule {
    pub param_value: u32, // Rust field that will be turned into a Verilog parameter
}

impl Logic for ParamModule {
    fn update(&mut self) {
        // Use param_value in logic
    }
}

fn set(param : u32) {
    let mut module = ParamModule::default();
    module.param_value = param; // Set before generating Verilog

    // Generate Verilog
    let verilog_code = generate_verilog(&module);
    println!("{}", verilog_code);
}
