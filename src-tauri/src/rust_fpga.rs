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

fn set(param1 : u16, param2 : u16) {
    let mut module = ParamModule::default();
    module.param_value = u16_to_u32(param1, param2); // Set before generating Verilog
    // Generate Verilog
    let verilog_code = generate_verilog(&module);
    println!("{}", verilog_code);
}

/// Combines two u16 values into a u32.
/// `high` becomes the upper 16 bits, `low` becomes the lower 16 bits.
fn u16_to_u32(high: u16, low: u16) -> u32 {
    ((high as u32) << 16) | (low as u32)
}

/// Splits a u32 into two u16 values.
/// Returns (high, low).
fn u32_to_u16(value: u32) -> (u16, u16) {
    let high = (value >> 16) as u16;
    let low = (value & 0xFFFF) as u16;
    (high, low)
}

#[cfg(test)]
mod tests {
    use super::*; // Imports everything from the outer module so you can test it


    #[test]
    fn test_u16_to_u32() {
        let result = u16_to_u32(0x1234, 0x5678);
        assert_eq!(result, 0x12345678);
    }

    #[test]
    fn test_u32_to_u16() {
        let (high, low) = u32_to_u16(0x12345678);
        assert_eq!(high, 0x1234);
        assert_eq!(low, 0x5678);
    }
}