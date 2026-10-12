use glassvm_core::{MachineId, SchemaRef, SchemaVersion};
use glassvm_normalizer_contract::InputId;
use glassvm_query::TraceQueryEngine;
use glassvm_recorder::ReferenceChannel;
use glassvm_registry::Registry;

fn main() {
    let machine = MachineId::from("chip8");
    let input = InputId::new("chip8.key.0").expect("canonical input ID");
    let schema = SchemaRef::new(
        "glassvm.input.digital_key",
        SchemaVersion::new(1, 0, 0),
    );
    let registry = Registry::new();
    let query = TraceQueryEngine::new(Vec::new()).expect("empty selected projection");
    let channel = ReferenceChannel::NormalizedEvents;

    assert_eq!(machine.as_str(), "chip8");
    assert_eq!(input.as_str(), "chip8.key.0");
    assert_eq!(schema.id, "glassvm.input.digital_key");
    assert_eq!(query.events().len(), 0);
    assert_eq!(registry.list().len(), 0);
    assert_eq!(channel, ReferenceChannel::NormalizedEvents);
    println!("external consumer built against all five GlassVM crates");
}
