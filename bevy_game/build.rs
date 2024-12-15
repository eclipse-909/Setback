extern crate embed_resource;

fn main() {
	let target: String = std::env::var("TARGET").unwrap();
	if target.contains("windows") {
		embed_resource::compile("icon.rc", embed_resource::NONE);
	}
}