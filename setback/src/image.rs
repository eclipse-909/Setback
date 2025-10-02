use {
	image::{
		codecs::gif::GifDecoder,
		RgbaImage, AnimationDecoder
	},
	std::{
		sync::Arc,
		io::Cursor
	},
	vello::{
		peniko::{self, Blob, ImageFormat},
		kurbo::Size
	}
};

pub struct Image {
	pub image_img: RgbaImage,
	pub peniko_img: peniko::Image
}

impl Image {
	fn size_image(image_img: RgbaImage, size: Size) -> Self {
		let (width, height) = image_img.dimensions();
		let peniko_img = if width != size.width as u32 || height != size.height as u32 {
			let width = size.width as u32;
			let height = size.height as u32;
			peniko::Image::new(Blob::new(Arc::new(image::imageops::resize(
				&image_img,
				width,
				height,
				image::imageops::FilterType::CatmullRom
			).into_raw())), ImageFormat::Rgba8, width, height)
		} else {
			peniko::Image::new(Blob::new(Arc::new(image_img.as_raw().clone())), ImageFormat::Rgba8, width, height)
		};
		Self {image_img, peniko_img}
	}
	
	pub fn load_png(bytes: &[u8]) -> Self {
		let image_img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap().into_rgba8();
		let (width, height) = image_img.dimensions();
		let peniko_img = peniko::Image::new(Blob::new(Arc::new(image_img.as_raw().clone())), ImageFormat::Rgba8, width, height);
		Self {image_img, peniko_img}
	}
	
	pub fn load_png_with_size(bytes: &[u8], size: Size) -> Self {
		Self::size_image(image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap().into_rgba8(), size)
	}
	
	pub fn load_png_with_scale(bytes: &[u8], scale: f32) -> Self {
		let image_img = image::load_from_memory_with_format(bytes, image::ImageFormat::Png).unwrap().into_rgba8();
		let (width, height) = image_img.dimensions();
		let peniko_img = if scale != 1.0 {
			let width = (width as f32 * scale) as u32;
			let height = (height as f32 * scale) as u32;
			peniko::Image::new(Blob::new(Arc::new(image::imageops::resize(
				&image_img,
				width,
				height,
				image::imageops::FilterType::CatmullRom
			).into_raw())), ImageFormat::Rgba8, width, height)
		} else {
			peniko::Image::new(Blob::new(Arc::new(image_img.as_raw().clone())), ImageFormat::Rgba8, width, height)
		};
		Self {image_img, peniko_img}
	}
	
	/// Try to have the sizes known at compile time so you don't have to use this expensive function at runtime.
	///
	/// Returns None if the size is the same as it was.
	///
	/// Prefer [Self::load_png_with_size] or [Self::load_png_with_scale] for initializing an image.
	pub fn resize(&mut self, size: Size) -> Option<peniko::Image> {
		if self.peniko_img.width == size.width as u32 && self.peniko_img.height == size.height as u32 {return None;}
		let resized = image::imageops::resize(
			&self.image_img,
			size.width as u32,
			size.height as u32,
			image::imageops::FilterType::CatmullRom
		);
		let (width, height) = resized.dimensions();
		Some(peniko::Image::new(Blob::new(Arc::new(resized.into_raw())), ImageFormat::Rgba8, width, height))
	}
}

pub fn images_from_gif_with_size(gif: &[u8], size: Size) -> Vec<Image> {
	GifDecoder::new(Cursor::new(gif))
		.unwrap()
		.into_frames()
		.map(|frame| Image::size_image(frame.unwrap().into_buffer(), size))
		.collect()
}