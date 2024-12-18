extern crate embed_resource;

fn main() {
	let target: String = std::env::var("TARGET").unwrap();
	if target.contains("windows") {
		let _ = embed_resource::compile("./game/icon.rc", embed_resource::NONE);
	}
}