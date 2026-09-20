//! Hierarchical addresses: ri (position in the world), then offsets at each level,
//! then the vertical layer.

use std::fmt;

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

pub fn address_of(shaku: Hex, layer: i32) -> Address {
    let ken = parent_of(shaku, Level::Shaku);
    let cho = parent_of(ken, Level::Ken);
    let ri = parent_of(cho, Level::Cho);
    Address {
        ri,
        cho: local(cho, Level::Cho),
        ken: local(ken, Level::Ken),
        shaku: local(shaku, Level::Shaku),
        layer,
    }
}

pub fn shaku_of(addr: &Address) -> (Hex, i32) {
    let cho = Hex::new(
        addr.ri.q * Level::Ri.packing() + addr.cho.q,
        addr.ri.r * Level::Ri.packing() + addr.cho.r,
    );
    let ken = Hex::new(
        cho.q * Level::Cho.packing() + addr.ken.q,
        cho.r * Level::Cho.packing() + addr.ken.r,
    );
    let shaku = Hex::new(
        ken.q * Level::Ken.packing() + addr.shaku.q,
        ken.r * Level::Ken.packing() + addr.shaku.r,
    );
    (shaku, addr.layer)
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ri ({},{}) / cho ({},{}) / ken ({},{}) / shaku ({},{}) / layer {}",
            self.ri.q,
            self.ri.r,
            self.cho.q,
            self.cho.r,
            self.ken.q,
            self.ken.r,
            self.shaku.q,
            self.shaku.r,
            self.layer
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
            Hex::new(1, -1),
            Hex::new(4_211, -9_003),
            Hex::new(-12_961, 7),
        ] {
            for layer in [-7, 0, 213] {
                let addr = address_of(shaku, layer);
                assert_eq!(shaku_of(&addr), (shaku, layer), "{shaku:?} {layer}");
            }
        }
    }

    #[test]
    fn the_origin_is_the_centre_of_everything() {
        let addr = address_of(Hex::ZERO, 0);
        assert_eq!(addr.ri, Hex::ZERO);
        assert_eq!(addr.cho, Hex::ZERO);
        assert_eq!(addr.ken, Hex::ZERO);
        assert_eq!(addr.shaku, Hex::ZERO);
    }

    #[test]
    fn neighbouring_shaku_across_a_ken_border_differ_in_ken() {
        // The shaku east of a ken's eastern edge belongs to the next ken.
        let a = Hex::new(2, 0);
        let b = Hex::new(3, 0);
        let addr_a = address_of(a, 0);
        let addr_b = address_of(b, 0);
        assert_ne!(addr_a.ken, addr_b.ken, "{addr_a} vs {addr_b}");
    }

    #[test]
    fn display_lists_every_part() {
        let text = format!("{}", address_of(Hex::new(7, -3), 12));
        assert!(text.contains("ri ("), "{text}");
        assert!(text.contains("cho ("), "{text}");
        assert!(text.contains("ken ("), "{text}");
        assert!(text.contains("shaku ("), "{text}");
        assert!(text.contains("layer 12"), "{text}");
    }
}
