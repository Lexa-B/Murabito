//! Hierarchical addresses: ri (position in the world), then offsets at each level,
//! then the vertical layer.

use std::fmt;

use murabito_hexcoords::VoxelCoord;

use crate::hex::Hex;
use crate::level::Level;
use crate::owner::{local, parent_of};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Address {
    /// The ri's position in the world.
    pub ri: Hex,
    /// The cho's offset from its ri's centre, in cho.
    pub cho: Hex,
    /// The ken's offset from its cho's centre, in ken.
    pub ken: Hex,
    /// The shaku's offset from its ken's centre, in shaku.
    pub shaku: Hex,
    pub layer: i32,
}

/// The address of a shaku voxel.
pub fn address_of(voxel: VoxelCoord) -> Address {
    let shaku = Hex::axial(voxel.q(), voxel.r());
    let ken = parent_of(shaku, Level::Shaku);
    let cho = parent_of(ken, Level::Ken);
    let ri = parent_of(cho, Level::Cho);
    Address {
        ri,
        cho: local(cho, Level::Cho),
        ken: local(ken, Level::Ken),
        shaku: local(shaku, Level::Shaku),
        layer: voxel.layer(),
    }
}

/// The shaku voxel an address names.
pub fn voxel_of(addr: &Address) -> VoxelCoord {
    let cho = Hex::axial(
        addr.ri.q() * Level::Ri.packing() + addr.cho.q(),
        addr.ri.r() * Level::Ri.packing() + addr.cho.r(),
    );
    let ken = Hex::axial(
        cho.q() * Level::Cho.packing() + addr.ken.q(),
        cho.r() * Level::Cho.packing() + addr.ken.r(),
    );
    let shaku = Hex::axial(
        ken.q() * Level::Ken.packing() + addr.shaku.q(),
        ken.r() * Level::Ken.packing() + addr.shaku.r(),
    );
    VoxelCoord::new(shaku.q(), shaku.r(), shaku.s(), addr.layer)
        .expect("a Hex is always on the plane")
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ri {} / cho {} / ken {} / shaku {} / layer {}",
            self.ri, self.cho, self.ken, self.shaku, self.layer
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses_round_trip() {
        for shaku in [
            Hex::ZERO,
            Hex::axial(1, -1),
            Hex::axial(4_211, -9_003),
            Hex::axial(-12_961, 7),
        ] {
            for layer in [-7, 0, 213] {
                let voxel = VoxelCoord::new(shaku.q(), shaku.r(), shaku.s(), layer).unwrap();
                assert_eq!(voxel_of(&address_of(voxel)), voxel, "{shaku:?} {layer}");
            }
        }
    }

    #[test]
    fn the_origin_is_the_centre_of_everything() {
        let addr = address_of(VoxelCoord::new(0, 0, 0, 0).unwrap());
        assert_eq!(addr.ri, Hex::ZERO);
        assert_eq!(addr.cho, Hex::ZERO);
        assert_eq!(addr.ken, Hex::ZERO);
        assert_eq!(addr.shaku, Hex::ZERO);
    }

    #[test]
    fn neighbouring_shaku_across_a_ken_border_differ_in_ken() {
        // The shaku east of a ken's eastern edge belongs to the next ken.
        let addr_a = address_of(VoxelCoord::new(2, 0, -2, 0).unwrap());
        let addr_b = address_of(VoxelCoord::new(3, 0, -3, 0).unwrap());
        assert_ne!(addr_a.ken, addr_b.ken, "{addr_a} vs {addr_b}");
    }

    #[test]
    fn display_lists_every_part() {
        let text = format!("{}", address_of(VoxelCoord::new(7, -3, -4, 12).unwrap()));
        assert!(text.contains("ri ("), "{text}");
        assert!(text.contains("cho ("), "{text}");
        assert!(text.contains("ken ("), "{text}");
        assert!(text.contains("shaku ("), "{text}");
        assert!(text.contains("layer 12"), "{text}");
    }
}
