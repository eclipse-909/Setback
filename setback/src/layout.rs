use {
	crate::{image::*, state::*, style::*, text::*},
	glyphon::FontSystem,
	libsetback::Arena,
	std::slice::Iter,
	taffy::{
		AvailableSpace, CacheTree, ClearState, Display, Layout, LayoutInput, LayoutOutput,
		LayoutPartialTree, NodeId, RoundTree, Size, TraversePartialTree, TraverseTree,
		compute_cached_layout, compute_flexbox_layout, compute_hidden_layout, compute_leaf_layout,
		compute_root_layout, round_layout, tree::Cache,
	},
	vello::peniko::Image,
};

pub struct Node<'buffer> {
	pub style: Style,
	unrounded_layout: Layout,
	pub final_layout: Layout,
	cache: Cache,
	pub inner: InnerElement<'buffer>,
	parent: Option<NodeId>,
	pub node_state: Option<Box<dyn NodeState>>,
}

impl Default for Node<'_> {
	fn default() -> Self {
		Node {
			style: Style::default(),
			cache: Cache::new(),
			unrounded_layout: Layout::with_order(0),
			final_layout: Layout::with_order(0),
			inner: InnerElement::None,
			parent: None,
			node_state: None,
		}
	}
}

pub enum InnerElement<'buffer> {
	Text(TextElement<'buffer>),
	Image(Image),
	Children(Vec<NodeId>),
	None,
}

impl<'buffer> From<Style> for Node<'buffer> {
	fn from(style: Style) -> Self {
		Self {
			style,
			..Default::default()
		}
	}
}

impl<'buffer> Node<'buffer> {
	pub fn push_child(&mut self, node: NodeId) {
		Self::from_id_mut(node).parent = Some(self.as_id());
		match &mut self.inner {
			InnerElement::None => self.inner = InnerElement::Children(vec![node]),
			InnerElement::Children(children) => children.push(node),
			InnerElement::Text(_) | InnerElement::Image(_) => {
				panic!("Can't have text or image with children nodes")
			}
		}
	}

	pub fn children(&self) -> Iter<'_, NodeId> {
		match &self.inner {
			InnerElement::Children(children) => children.iter(),
			InnerElement::None | InnerElement::Text(_) | InnerElement::Image(_) => [].iter(),
		}
	}

	#[inline(always)]
	fn as_id(&self) -> NodeId {
		NodeId::from(self as *const Node as usize)
	}
	#[inline(always)]
	pub fn from_id<'a>(node_id: NodeId) -> &'a Node<'buffer> {
		unsafe { &*(usize::from(node_id) as *const Node) }
	}
	#[inline(always)]
	pub fn from_id_mut<'a>(node_id: NodeId) -> &'a mut Node<'buffer> {
		unsafe { &mut *(usize::from(node_id) as *mut Node) }
	}

	#[inline]
	pub fn mark_dirty(&mut self) {
		if let ClearState::Cleared = self.cache.clear()
			&& let Some(parent) = self.parent
		{
			Node::from_id_mut(parent).mark_dirty();
		}
	}
}

pub struct ChildIter<'a>(Iter<'a, NodeId>);
impl Iterator for ChildIter<'_> {
	type Item = NodeId;
	fn next(&mut self) -> Option<Self::Item> {
		self.0.next().copied()
	}
}

pub struct LayoutTree {
	pub font_system: FontSystem,
	nodes: Vec<NodeId>,
	stack: Vec<NodeId>,
}

impl LayoutTree {
	pub const fn new(font_system: FontSystem) -> Self {
		Self {
			font_system,
			nodes: Vec::new(),
			stack: Vec::new(),
		}
	}

	pub fn new_node(&mut self, arena: &Arena, style: Style) -> NodeId {
		let node = NodeId::new(arena.alloc(Node::from(style)) as *mut Node as u64);
		self.nodes.reserve(1);
		self.stack.reserve(1);
		node
	}

	pub fn compute_layout(
		&mut self,
		root_node: NodeId,
		available_space: Size<AvailableSpace>,
	) -> &[NodeId] {
		compute_root_layout(self, root_node, available_space);
		round_layout(self, root_node);

		// order nodes
		self.nodes.clear();
		self.stack.clear();
		self.stack.push(root_node);
		while let Some(node) = self.stack.pop() {
			self.nodes.push(node);
			for child in Node::from_id(node).children().rev() {
				self.stack.push(*child);
			}
		}
		self.nodes.sort_by(|node_a, node_b| {
			let node_a = Node::from_id(*node_a);
			let node_b = Node::from_id(*node_b);
			(node_a.style.z_index, node_a.final_layout.order)
				.cmp(&(node_b.style.z_index, node_b.final_layout.order))
		});
		self.nodes.as_slice()
	}
}

impl TraversePartialTree for LayoutTree {
	type ChildIter<'a>
		= ChildIter<'a>
	where
		Self: 'a;
	fn child_ids(&self, node_id: NodeId) -> Self::ChildIter<'_> {
		ChildIter(match &Node::from_id(node_id).inner {
			InnerElement::Children(children) => children.iter(),
			InnerElement::Text(_) | InnerElement::Image(_) | InnerElement::None => [].iter(),
		})
	}
	fn child_count(&self, node_id: NodeId) -> usize {
		match &Node::from_id(node_id).inner {
			InnerElement::Children(children) => children.len(),
			InnerElement::Text(_) | InnerElement::Image(_) | InnerElement::None => 0,
		}
	}
	fn get_child_id(&self, node_id: NodeId, index: usize) -> NodeId {
		match &Node::from_id(node_id).inner {
			InnerElement::Children(children) => children[index],
			InnerElement::Text(_) | InnerElement::Image(_) | InnerElement::None => {
				panic!("Node has no children")
			}
		}
	}
}

impl TraverseTree for LayoutTree {}

impl LayoutPartialTree for LayoutTree {
	type CoreContainerStyle<'a>
		= &'a Style
	where
		Self: 'a;
	type CustomIdent = winit::keyboard::SmolStr;

	fn get_core_container_style(&self, node_id: NodeId) -> Self::CoreContainerStyle<'_> {
		&Node::from_id(node_id).style
	}

	fn resolve_calc_value(&self, _val: *const (), _basis: f32) -> f32 {
		0.0
	}

	fn set_unrounded_layout(&mut self, node_id: NodeId, layout: &Layout) {
		Node::from_id_mut(node_id).unrounded_layout = *layout;
	}

	fn compute_child_layout(&mut self, node_id: NodeId, inputs: LayoutInput) -> LayoutOutput {
		compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
			let node = Node::from_id_mut(node_id);
			match (node.style.display, tree.child_count(node_id) > 0) {
				(Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
				(Display::None, true) => compute_hidden_layout(tree, node_id),
				(_, false) => compute_leaf_layout(
					inputs,
					&node.style,
					|_, _| 0.,
					|known_dimensions, available_space| match &mut node.inner {
						InnerElement::Text(text) => {
							text.measure(known_dimensions, available_space, &mut tree.font_system)
						}
						InnerElement::Image(img) => measure_image(img, known_dimensions),
						InnerElement::None => Size::ZERO,
						InnerElement::Children(_) => unreachable!(),
					},
				),
			}
		})
	}
}

impl CacheTree for LayoutTree {
	fn cache_get(
		&self,
		node_id: NodeId,
		known_dimensions: Size<Option<f32>>,
		available_space: Size<AvailableSpace>,
		run_mode: taffy::RunMode,
	) -> Option<LayoutOutput> {
		Node::from_id(node_id)
			.cache
			.get(known_dimensions, available_space, run_mode)
	}
	fn cache_store(
		&mut self,
		node_id: NodeId,
		known_dimensions: Size<Option<f32>>,
		available_space: Size<AvailableSpace>,
		run_mode: taffy::RunMode,
		layout_output: LayoutOutput,
	) {
		Node::from_id_mut(node_id).cache.store(
			known_dimensions,
			available_space,
			run_mode,
			layout_output,
		)
	}
	fn cache_clear(&mut self, node_id: NodeId) {
		Node::from_id_mut(node_id).cache.clear();
	}
}
impl taffy::LayoutFlexboxContainer for LayoutTree {
	type FlexboxContainerStyle<'a>
		= &'a Style
	where
		Self: 'a;
	type FlexboxItemStyle<'a>
		= &'a Style
	where
		Self: 'a;
	fn get_flexbox_container_style(&self, node_id: NodeId) -> Self::FlexboxContainerStyle<'_> {
		&Node::from_id(node_id).style
	}
	fn get_flexbox_child_style(&self, child_node_id: NodeId) -> Self::FlexboxItemStyle<'_> {
		&Node::from_id(child_node_id).style
	}
}

impl RoundTree for LayoutTree {
	fn get_unrounded_layout(&self, node_id: NodeId) -> Layout {
		Node::from_id_mut(node_id).unrounded_layout
	}
	fn set_final_layout(&mut self, node_id: NodeId, layout: &Layout) {
		Node::from_id_mut(node_id).final_layout = *layout;
	}
}
