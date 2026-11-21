//! Review 1, security reviewer: identity integrity checks of `surf-ports` through its public API.

use std::io;

use surf_ports::{DocumentPorts, NavigatorPorts, RandomSource, RendererPorts, ShellPorts, shell_port};
use surf_types::behaviour::TryContext;
use surf_types::document::DocumentId;
use surf_types::net::RequestId;
use surf_types::obligation::UnmetObligation;
use surf_types::primitives::Uuid;
use surf_types::render::FrameId;
use surf_types::shell::{OpenWindow, TabId, WindowId};

/// Records the buffer it is handed on entry, then fills it with a marker byte.
struct Recording {
    seen: Vec<Vec<u8>>,
}

impl RandomSource for Recording {
    fn fill(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        self.seen.push(buffer.to_vec());
        buffer.fill(0xa5);
        Ok(())
    }
}

fn all_five<P: TryContext>(ports: &mut P) -> Vec<String> {
    let DocumentId(Uuid(a)) = ports.try_generate_surf_document_document_id().expect("id");
    let RequestId(Uuid(b)) = ports.try_generate_surf_net_request_id().expect("id");
    let FrameId(Uuid(c)) = ports.try_generate_surf_render_frame_id().expect("id");
    let TabId(Uuid(d)) = ports.try_generate_surf_shell_tab_id().expect("id");
    let WindowId(Uuid(e)) = ports.try_generate_surf_shell_window_id().expect("id");
    vec![a, b, c, d, e]
}

/// Each identity asks for its own 16 bytes in a buffer that holds nothing of the previous one.
#[test]
fn review1_every_identity_reads_16_bytes_into_a_fresh_zeroed_buffer() {
    let mut source = Recording { seen: Vec::new() };
    all_five(&mut ShellPorts::with_source(&mut source));
    all_five(&mut NavigatorPorts::with_source(&mut source));
    all_five(&mut DocumentPorts::with_source(&mut source));
    all_five(&mut RendererPorts::with_source(&mut source));
    assert_eq!(source.seen.len(), 20);
    for (at, buffer) in source.seen.iter().enumerate() {
        assert_eq!(buffer, &vec![0u8; 16], "identity {at} was handed {buffer:?}");
    }
}

/// Fills from a 128-bit pattern.
struct Pattern(u128);

impl RandomSource for Pattern {
    fn fill(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        buffer.copy_from_slice(&self.0.to_be_bytes()[..buffer.len()]);
        Ok(())
    }
}

fn identity(pattern: u128) -> String {
    let WindowId(Uuid(text)) = ShellPorts::with_source(Pattern(pattern))
        .try_generate_surf_shell_window_id()
        .expect("id");
    text
}

/// Exactly 122 of the 128 source bits reach the identity, and no two of them land in the same
/// place: flipping one source bit changes the text in exactly the six fixed bits' absence.
#[test]
fn review1_identity_carries_exactly_122_source_bits() {
    let zero = identity(0);
    let mut carried = std::collections::BTreeSet::new();
    for bit in 0..128 {
        let flipped = identity(1u128 << (127 - bit));
        if flipped != zero {
            assert!(carried.insert(flipped.clone()), "bit {bit} collides");
        }
    }
    assert_eq!(carried.len(), 122);
}

/// A source that writes part of the buffer and then fails yields no identity and no row.
struct Short;

impl RandomSource for Short {
    fn fill(&mut self, buffer: &mut [u8]) -> io::Result<()> {
        buffer[..8].fill(0x11);
        Err(io::Error::from(io::ErrorKind::UnexpectedEof))
    }
}

#[test]
fn review1_a_short_fill_is_a_failure_and_never_a_padded_identity() {
    let mut shell = shell_port(ShellPorts::with_source(Short));
    assert_eq!(
        shell.open_window(OpenWindow {}),
        Err(UnmetObligation {
            capability: "context answer",
            source: "surf.shell.WindowId"
        })
    );
    assert_eq!(shell.windows(), Ok(Vec::new()));
}

/// The default source: over 2,000 identities from separately built ports, every one of the 122
/// free bits is set in between 35 % and 65 % of them (more than 13 standard deviations), and the
/// six fixed bits are always the same.
#[test]
fn review1_os_source_feeds_every_free_bit() {
    let n = 2000;
    let mut ones = [0usize; 128];
    for _ in 0..n {
        let WindowId(Uuid(text)) = ShellPorts::new()
            .try_generate_surf_shell_window_id()
            .expect("id");
        let hex: String = text.chars().filter(|c| *c != '-').collect();
        let value = u128::from_str_radix(&hex, 16).expect("hex");
        for (bit, count) in ones.iter_mut().enumerate() {
            *count += usize::from(value >> (127 - bit) & 1 == 1);
        }
    }
    let fixed: Vec<usize> = (0..128).filter(|&b| ones[b] == 0 || ones[b] == n).collect();
    assert_eq!(fixed, vec![48, 49, 50, 51, 64, 65]);
    for (bit, &count) in ones.iter().enumerate() {
        if !fixed.contains(&bit) {
            assert!((700..=1300).contains(&count), "bit {bit} set {count} of {n} times");
        }
    }
}
