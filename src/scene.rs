// scene data, bvh

use glam::Vec3;
use std::fmt::{self, Debug};

// Id types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PrimitiveId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BvhId(pub usize);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MaterialId(pub usize);

// Primitives

#[derive(Clone)]
pub enum Primitive {
    Triangle(Triangle),
    Sphere(Sphere),
}
impl Primitive {
    pub fn material(&self) -> MaterialId {
        match self {
            Self::Triangle(t) => t.material,
            Self::Sphere(s) => s.material,
        }
    }
    pub fn center(&self) -> Vec3 {
        match self {
            Self::Triangle(t) => t.center(),
            Self::Sphere(s) => s.center,
        }
    }
    pub fn aabb(&self) -> Aabb {
        match self {
            Self::Triangle(t) => t.aabb(),
            Self::Sphere(s) => s.aabb(),
        }
    }
    pub fn raw(&self) -> [u8; 64] {
        match self {
            Self::Triangle(t) => t.raw(),
            Self::Sphere(s) => s.raw(),
        }
    }
    pub fn empty_raw() -> [u8; 64] {
        Triangle::new().raw()
    }
}
impl Debug for Primitive {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Triangle(t) => t.fmt(f),
            Self::Sphere(s) => s.fmt(f),
        }
    }
}

#[derive(Clone)]
pub struct Triangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    pub material: MaterialId,
}
impl Triangle {
    pub fn new() -> Self {
        Self {
            v0: Vec3::new(0.0, 0.0, 0.0),
            v1: Vec3::new(0.0, 0.0, 0.0),
            v2: Vec3::new(0.0, 0.0, 0.0),
            material: MaterialId(0),
        }
    }
    pub fn center(&self) -> Vec3 {
        (self.v0 + self.v1 + self.v2) / 3.0
    }
    pub fn aabb(&self) -> Aabb {
        Aabb::from_points(&[self.v0, self.v1, self.v2])
    }
    pub fn raw(&self) -> [u8; 64] {
        let mut ret = [0u8; 64];
        ret[00..12].copy_from_slice(bytemuck::cast_slice(&self.v0.to_array()));
        ret[12..16].copy_from_slice(bytemuck::cast_slice(&[0u32]));
        ret[16..28].copy_from_slice(bytemuck::cast_slice(&self.v1.to_array()));
        ret[28..32].copy_from_slice(bytemuck::cast_slice(&[
            u32::try_from(self.material.0).expect("material id exceeds u32")
        ]));
        ret[32..44].copy_from_slice(bytemuck::cast_slice(&self.v2.to_array()));
        ret[44..64].copy_from_slice(bytemuck::cast_slice(&[0f32; 5]));
        ret
    }
    pub fn primitive(self) -> Primitive {
        Primitive::Triangle(self)
    }
}
impl Debug for Triangle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Triangle {:?}, {:?}, {:?}", self.v0, self.v1, self.v2)
    }
}

#[derive(Clone)]
pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: MaterialId,
}
impl Sphere {
    pub fn new() -> Self {
        Self {
            center: Vec3::new(0.0, 0.0, 0.0),
            radius: 0.0,
            material: MaterialId(0),
        }
    }
    pub fn aabb(&self) -> Aabb {
        Aabb {
            min: self.center - Vec3::new(self.radius, self.radius, self.radius),
            max: self.center + Vec3::new(self.radius, self.radius, self.radius),
        }
    }
    pub fn raw(&self) -> [u8; 64] {
        let mut ret = [0u8; 64];
        ret[00..12].copy_from_slice(bytemuck::cast_slice(&self.center.to_array()));
        ret[12..16].copy_from_slice(bytemuck::cast_slice(&[1u32]));
        ret[16..28].copy_from_slice(bytemuck::cast_slice(&[self.radius as f32, 0f32, 0f32]));
        ret[28..32].copy_from_slice(bytemuck::cast_slice(&[
            u32::try_from(self.material.0).expect("material ID exceeds u32")
        ]));
        ret[32..64].copy_from_slice(bytemuck::cast_slice(&[0f32; 8]));
        ret
    }
    pub fn primitive(self) -> Primitive {
        Primitive::Sphere(self)
    }
}
impl Debug for Sphere {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Sphere center {:?} radius {:?}",
            self.center, self.radius
        )
    }
}

// AABB

const MARGIN: f32 = 0.00001;
const MARGIN_V: Vec3 = Vec3::new(MARGIN, MARGIN, MARGIN);

#[derive(Clone, Debug, Copy)]
pub struct Aabb {
    pub min: Vec3,
    pub max: Vec3,
}
impl Aabb {
    pub fn new() -> Self {
        Self {
            min: Vec3::new(0.0, 0.0, 0.0),
            max: Vec3::new(0.0, 0.0, 0.0),
        }
    }
    pub fn from_points(points: &[Vec3]) -> Self {
        let first = match points.first() {
            Some(p) => p,
            None => return Self::new(),
        };
        let mut min = *first;
        let mut max = *first;
        for point in points.into_iter().skip(1) {
            min = min.min(*point);
            max = max.max(*point);
        }
        Self { min, max }
    }
    pub fn from_aabbs(aabbs: &[Aabb]) -> Self {
        Self::from_aabb_iter(aabbs.iter().cloned())
    }
    pub fn from_aabb_iter<I>(aabbs: I) -> Self
    where
        I: IntoIterator<Item = Aabb>,
    {
        let mut iter = aabbs.into_iter();

        let Some(first) = iter.next() else {
            return Self::new();
        };

        let mut min = first.min;
        let mut max = first.max;

        for aabb in iter {
            min = min.min(aabb.min);
            max = max.max(aabb.max);
        }

        Self { min, max }
    }
    pub fn from_primitives(primitives: &[Primitive], ids: &[PrimitiveId]) -> Self {
        Self::from_aabb_iter(ids.iter().map(|id| primitives[id.0].aabb()))
    }
    pub fn with_margin(&self) -> Self {
        Aabb {
            min: self.min - MARGIN_V,
            max: self.max + MARGIN_V,
        }
    }
}

// BVH
// todo: multiple primitives, count as negative right index

#[derive(Clone, Debug)]
pub struct Bvh {
    pub aabb: Aabb,
    pub left: Option<BvhId>,
    pub right: Option<BvhId>,
    pub primitive: Option<PrimitiveId>,
}
impl Bvh {
    pub fn new() -> Self {
        Self {
            aabb: Aabb::new(),
            left: None,
            right: None,
            primitive: None,
        }
    }

    pub fn build(primitives: &[Primitive]) -> Vec<Self> {
        if primitives.is_empty() {
            return vec![Bvh::new()];
        }

        let ids: Vec<PrimitiveId> = (0..primitives.len()).map(|u| PrimitiveId(u)).collect();
        let mut nodes = Vec::new();

        Self::build_node(primitives, &ids, &mut nodes);

        nodes
    }

    fn build_node(primitives: &[Primitive], ids: &[PrimitiveId], nodes: &mut Vec<Bvh>) -> BvhId {
        let aabb = Aabb::from_primitives(primitives, ids).with_margin();
        let node_id = BvhId(nodes.len());
        nodes.push(Bvh {
            aabb,
            left: None,
            right: None,
            primitive: None,
        });

        if ids.len() <= 1 {
            nodes[node_id.0].primitive = ids.get(0).copied();
            return node_id;
        }

        // todo: surface area heuristic
        let size = aabb.max - aabb.min;
        let split_dim = if size.x >= size.y && size.x >= size.z {
            0
        } else if size.y >= size.z {
            1
        } else {
            2
        };

        let mut sorted_ids = ids.to_vec();
        sorted_ids.sort_by(|a, b| {
            primitives[a.0].center()[split_dim].total_cmp(&primitives[b.0].center()[split_dim])
        });

        let split = ids.len() / 2;

        let left = Self::build_node(primitives, &sorted_ids[..split], nodes);
        let right = Self::build_node(primitives, &sorted_ids[split..], nodes);

        nodes[node_id.0].left = Some(left);
        nodes[node_id.0].right = Some(right);

        node_id
    }

    pub fn raw(&self) -> [u8; 32] {
        let left_or_obj = self.left.map_or_else(
            || {
                -(self
                    .primitive
                    .expect("bvh must have a primitive or children")
                    .0 as i32)
                    - 1
            },
            |left| i32::try_from(left.0).expect("bvh id exceeds i32"),
        );

        let right = self.right.map_or(-1i32, |right| {
            i32::try_from(right.0).expect("bvh id exceeds i32")
        });

        let mut ret = [0u8; 32];
        ret[00..12].copy_from_slice(bytemuck::cast_slice(&self.aabb.min.to_array()));
        ret[12..16].copy_from_slice(bytemuck::cast_slice(&[left_or_obj]));
        ret[16..28].copy_from_slice(bytemuck::cast_slice(&self.aabb.max.to_array()));
        ret[28..32].copy_from_slice(bytemuck::cast_slice(&[right]));
        ret
    }

    pub fn empty_raw() -> [u8; 32] {
        Bvh::new().raw()
    }

    pub fn with_context<'a>(&'a self, scene: &'a Scene) -> BvhContext<'a> {
        BvhContext { bvh: self, scene }
    }

    pub fn fmt_with_indent(
        &self,
        f: &mut fmt::Formatter<'_>,
        depth: usize,
        scene: &Scene,
    ) -> fmt::Result {
        let indent = "    ";
        let pad = indent.repeat(depth);

        if let Some(id) = &self.primitive {
            writeln!(f, "{pad}BVH Leaf: {{")?;
            writeln!(f, "{pad}{indent}{:?}", self.aabb)?;
            writeln!(f, "{pad}{indent}{:?}", scene.primitives.get(id.0))?;
            write!(f, "{pad}}}")?;
            return Ok(());
        }

        writeln!(f, "{pad}BVH: {{")?;
        writeln!(f, "{pad}{indent}{:?}", self.aabb)?;
        if let Some(left) = &self.left {
            scene.bvhs[left.0].fmt_with_indent(f, depth + 1, scene)?;
        } else {
            writeln!(f, "{pad}{indent}<none>")?;
        }
        writeln!(f, "\n{pad}{indent}AND")?;
        if let Some(right) = &self.right {
            scene.bvhs[right.0].fmt_with_indent(f, depth + 1, scene)?;
        } else {
            writeln!(f, "{pad}{indent}<none>")?;
        }
        write!(f, "\n{pad}}}")
    }
}

pub struct BvhContext<'a> {
    bvh: &'a Bvh,
    scene: &'a Scene,
}
impl<'a> Debug for BvhContext<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.bvh.fmt_with_indent(f, 0, self.scene)
    }
}

// lambertian
pub enum Material {
    Lambertian(Lambertian),
}
impl Material {
    pub fn raw(&self) -> [u8; 64] {
        match self {
            Self::Lambertian(l) => l.raw(),
        }
    }
    pub fn empty_raw() -> [u8; 64] {
        Lambertian::new().raw()
    }
}

#[derive(Clone, Debug)]
pub struct Lambertian {
    pub color: Vec3,
}
impl Lambertian {
    pub fn new() -> Self {
        Self {
            color: Vec3::new(0.0, 0.0, 0.0),
        }
    }
    pub fn raw(&self) -> [u8; 64] {
        let mut ret = [0u8; 64];
        ret[00..12].copy_from_slice(bytemuck::cast_slice(&self.color.to_array()));
        ret[12..16].copy_from_slice(bytemuck::cast_slice(&[0u32]));
        ret[16..64].copy_from_slice(bytemuck::cast_slice(&[0f32; 12]));
        ret
    }
    pub fn material(self) -> Material {
        Material::Lambertian(self)
    }
}

pub struct Scene {
    pub bvhs: Vec<Bvh>,
    pub primitives: Vec<Primitive>,
    pub materials: Vec<Material>,
}
impl Scene {
    pub fn new() -> Self {
        Self {
            bvhs: vec![Bvh::new()],
            primitives: vec![],
            materials: vec![],
        }
    }
    pub fn from_data(primitives: Vec<Primitive>, materials: Vec<Material>) -> Self {
        for primitive in &primitives {
            materials
                .get(primitive.material().0)
                .expect("material id is invalid");
        }
        Self {
            bvhs: Bvh::build(&primitives),
            primitives,
            materials,
        }
    }
    pub fn rebuild_bvh(&mut self) {
        self.bvhs = Bvh::build(&self.primitives);
    }
}
