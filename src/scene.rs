// scene data, bvh

use dyn_clone::{DynClone, clone_trait_object};
use glam::Vec3;

// Primitives
// todo: make primitive a trait?

pub trait Primitive: DynClone {
    fn get_center(&self) -> Vec3;
    fn get_aabb(&self) -> Aabb;
    fn to_string(&self) -> String;
    fn to_raw(&self) -> Vec<f32>;
}
clone_trait_object!(Primitive);

pub struct Triangle {
    pub v0: Vec3,
    pub v1: Vec3,
    pub v2: Vec3,
    pub material: i32,
}
impl Primitive for Triangle {
    fn get_center(&self) -> Vec3 {
        (self.v0 + self.v1 + self.v2) / 3.0
    }
    fn get_aabb(&self) -> Aabb {
        Aabb::from_points(&vec![self.v0, self.v1, self.v2])
    }
    fn to_string(&self) -> String {
        format!(
            "Triangle {}, {}, {}",
            self.v0.to_string(),
            self.v1.to_string(),
            self.v2.to_string()
        )
    }
    fn to_raw(&self) -> Vec<f32> {
        todo!()
    }
}
impl Clone for Triangle {
    fn clone(&self) -> Self {
        Self {
            v0: self.v0,
            v1: self.v1,
            v2: self.v2,
            material: self.material,
        }
    }
}

pub struct Sphere {
    pub center: Vec3,
    pub radius: f32,
    pub material: i32,
}
impl Primitive for Sphere {
    fn get_center(&self) -> Vec3 {
        self.center
    }
    fn get_aabb(&self) -> Aabb {
        Aabb::from_dims(
            self.center - Vec3::new(self.radius, self.radius, self.radius),
            Vec3::new(self.radius * 2.0, self.radius * 2.0, self.radius * 2.0),
        )
    }
    fn to_string(&self) -> String {
        format!(
            "Sphere center {} radius {}",
            self.center.to_string(),
            self.radius.to_string()
        )
    }
    fn to_raw(&self) -> Vec<f32> {
        let mut res = vec![];
        res.extend(vec![self.center.x, self.center.y, self.center.z, 1.0]);
        res.extend(vec![self.radius, 0.0, 0.0, self.material as f32]);
        res.extend(vec![0.0; 8]);
        res
    }
}
impl Clone for Sphere {
    fn clone(&self) -> Self {
        Self {
            center: self.center,
            radius: self.radius,
            material: self.material,
        }
    }
}

// AABB

const MARGIN: f32 = 0.00001;
const MARGIN_V: Vec3 = Vec3::new(MARGIN, MARGIN, MARGIN);

pub struct Aabb {
    pub pos: Vec3,
    pub size: Vec3,
}
impl Aabb {
    pub fn new() -> Self {
        Self {
            pos: -MARGIN_V,
            size: MARGIN_V * 2.0,
        }
    }
    pub fn from_dims(pos: Vec3, size: Vec3) -> Self {
        Self {
            pos: pos - MARGIN_V,
            size: size + MARGIN_V * 2.0,
        }
    }
    pub fn from_points(points: &Vec<Vec3>) -> Self {
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
        Self::from_dims(min, max - min)
    }
    pub fn from_aabbs(aabbs: &Vec<Aabb>) -> Self {
        let first = match aabbs.first() {
            Some(p) => p,
            None => return Self::new(),
        };
        let mut min = first.pos;
        let mut max = first.pos + first.size;
        for aabb in aabbs.iter().skip(1) {
            min = min.min(aabb.pos);
            max = max.max(aabb.pos + aabb.size);
        }
        Self {
            pos: min,
            size: max - min,
        }
    }
    pub fn from_primitives(primitives: &Vec<Box<dyn Primitive>>) -> Self {
        Self::from_aabbs(&primitives.iter().map(|prim| prim.get_aabb()).collect())
    }
}

// BVH

pub struct Bvh {
    pub aabb: Aabb,
    pub left: Option<Box<Bvh>>,
    pub right: Option<Box<Bvh>>,
    pub primitives: Vec<Box<dyn Primitive>>,
}
impl Bvh {
    // TODO: use indices instead of references, store all bvhs in a list
    pub fn from_primitives(primitives: &Vec<Box<dyn Primitive>>) -> Self {
        let aabb = Aabb::from_primitives(primitives);
        if primitives.len() < 2 {
            return Self {
                aabb,
                left: None,
                right: None,
                primitives: primitives.clone(),
            };
        }

        // TODO: surface area heuristic
        let split_dim = if aabb.size.x >= aabb.size.y && aabb.size.x >= aabb.size.z {
            0
        } else if aabb.size.y >= aabb.size.z {
            1
        } else {
            2
        };

        let mut primitive_is: Vec<(usize, f32)> = primitives
            .iter()
            .enumerate()
            .map(|(i, prim)| (i, prim.get_center()[split_dim]))
            .collect();
        primitive_is.sort_by(|a, b| a.1.total_cmp(&b.1));

        let split_i = primitives.len() / 2;

        let primitive_is_left: Vec<usize> =
            primitive_is[0..split_i].iter().map(|(i, _)| *i).collect();
        let primitive_is_right: Vec<usize> =
            primitive_is[split_i..].iter().map(|(i, _)| *i).collect();

        let bvh_left = if primitive_is_left.len() == 0 {
            None
        } else {
            Some(Box::new(Bvh::from_primitives(
                &primitive_is_left
                    .iter()
                    .map(|i| {
                        primitives
                            .get(*i)
                            .expect("range created value can't be out of bounds")
                            .clone()
                    })
                    .collect(),
            )))
        };
        let bvh_right = if primitive_is_right.len() == 0 {
            None
        } else {
            Some(Box::new(Bvh::from_primitives(
                &primitive_is_right
                    .iter()
                    .map(|i| {
                        primitives
                            .get(*i)
                            .expect("range created value can't be out of bounds")
                            .clone()
                    })
                    .collect(),
            )))
        };

        Self {
            primitives: vec![],
            aabb,
            left: bvh_left,
            right: bvh_right,
        }
    }
    pub fn to_string(&self) -> String {
        let indent = "    ";
        if self.primitives.len() > 0 {
            return format!(
                "BVH Leaf: {{\n{}\n}}",
                self.primitives
                    .iter()
                    .map(|prim| format!("{}{}", indent, prim.to_string()))
                    .collect::<Vec<String>>()
                    .join("\n")
            );
        }
        let left = match &self.left {
            Some(bvh) => bvh.to_string(),
            None => "<none>".to_string(),
        };
        let right = match &self.right {
            Some(bvh) => bvh.to_string(),
            None => "<none>".to_string(),
        };
        let total = format!("{}\nAND\n{}", left, right)
            .lines()
            .map(|line| format!("{}{}", indent, line))
            .collect::<Vec<String>>()
            .join("\n");
        return format!("BVH: {{\n{}\n}}", total);
    }
}

// lambertian
// todo: material as trait
pub trait Material: DynClone {
    fn to_raw(&self) -> Vec<f32>;
}
clone_trait_object!(Material);

pub struct Lambertian {
    pub color: Vec3,
}
impl Material for Lambertian {
    fn to_raw(&self) -> Vec<f32> {
        let mut res = vec![];
        res.extend_from_slice(&self.color.to_array());
        res.extend(vec![0.0]);
        res.extend(vec![0.0; 12]);
        res
    }
}
impl Clone for Lambertian {
    fn clone(&self) -> Self {
        Self { color: self.color }
    }
}

pub fn test() {
    let primitives: Vec<Box<dyn Primitive>> = vec![
        Box::new(Triangle {
            v0: Vec3::new(0.0, 0.0, 0.0),
            v1: Vec3::new(1.0, 0.0, 0.0),
            v2: Vec3::new(0.0, 1.0, 0.0),
            material: 0,
        }),
        Box::new(Triangle {
            v0: Vec3::new(5.0, 0.0, 0.0),
            v1: Vec3::new(6.0, 0.0, 0.0),
            v2: Vec3::new(5.0, 1.0, 0.0),
            material: 0,
        }),
        Box::new(Sphere {
            center: Vec3::new(10.0, 0.0, 0.0),
            radius: 0.0,
            material: 0,
        }),
        Box::new(Triangle {
            v0: Vec3::new(3.0, 5.0, 0.0),
            v1: Vec3::new(4.0, 5.0, 0.0),
            v2: Vec3::new(3.0, 6.0, 0.0),
            material: 0,
        }),
    ];
    let bvh = Bvh::from_primitives(&primitives);
    println!("{}", bvh.to_string());
}

pub struct Scene {
    pub objects: Vec<Box<dyn Primitive>>,
    // pub bvh
    pub materials: Vec<Box<dyn Material>>,
}
