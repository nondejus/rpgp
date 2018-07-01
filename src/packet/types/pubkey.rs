use errors::Result;
use key::Key;
use packet::types::{Signature, User, UserAttribute};
use packet::{tags, Packet, Tag};

fn take_sigs(packets: &[Packet], mut ctr: usize) -> Vec<Signature> {
    let mut res = vec![];
    while ctr < packets.len() && packets[ctr].tag == Tag::Signature {
        let sig_res = tags::sig::parser(packets[ctr].body.as_slice());
        // TODO: error handling
        if sig_res.is_err() {
            println!("failed to parse sig: {:?}", sig_res);
        }
        let (_, sig) = sig_res.unwrap();
        res.push(sig);
        ctr += 1;
    }

    res
}

/// Parse a transferable public key
/// Ref: https://tools.ietf.org/html/rfc4880.html#section-11.1
fn parse_single(mut ctr: usize, packets: &[Packet]) -> Result<(usize, Key)> {
    let packets_len = packets.len();

    // -- One Public-Key packet
    // TODO: better error management
    assert_eq!(packets[ctr].tag, Tag::PublicKey);

    let res = tags::pubkey::parser(packets[ctr].body.as_slice());
    if res.is_err() {
        println!("failed to parse pubkey {:?}", &res);
        println!("{:?}", packets[ctr]);
    }
    let (_, primary_key) = res?;

    ctr += 1;

    // -- Zero or more revocation signatures
    let rev_sigs = take_sigs(packets, ctr);
    ctr += rev_sigs.len();

    // -- Zero or more User ID packets
    // -- Zero or more User Attribute packets

    let mut users = vec![];
    let mut user_attrs = vec![];

    while ctr < packets_len {
        match packets[ctr].tag {
            Tag::UserID => {
                // TODO: better erorr handling
                let id = tags::userid::parser(packets[ctr].body.as_slice());
                ctr += 1;

                // --- zero or more signature packets
                let sigs = take_sigs(packets, ctr);
                ctr += sigs.len();

                users.push(User::new(id, sigs));
            }
            Tag::UserAttribute => {
                // TODO: better error handling
                let a = tags::userattr::parser(packets[ctr].body.as_slice());
                if a.is_err() {
                    println!("failed to parse {:?}\n{:?}", packets[ctr], a);
                }

                let (_, attr) = a?;
                ctr += 1;

                // --- zero or more signature packets
                let sigs = take_sigs(packets, ctr);
                ctr += sigs.len();

                user_attrs.push(UserAttribute::new(attr, sigs));
            }
            _ => break,
        }
    }

    // -- Zero or more Subkey packets
    let mut subkeys = vec![];
    while ctr < packets_len && packets[ctr].tag == Tag::PublicSubkey {
        // TODO: parse subkey
        let subkey = &packets[ctr];
        ctr += 1;

        let sigs = take_sigs(packets, ctr);
        ctr += sigs.len();

        // TODO: better error handling
        assert!(!sigs.is_empty(), "Missing signature");

        subkeys.push((subkey, sigs));
    }

    // TODO: better error handling
    assert!(!users.is_empty(), "Missing user ids");

    Ok((
        ctr,
        Key {
            primary_key,
            users,
            user_attributes: user_attrs,
            // TODO: subkeys
        },
    ))
}

/// Parse a transferable public key
/// Ref: https://tools.ietf.org/html/rfc4880.html#section-11.1
pub fn parse(packets: &[Packet]) -> Result<Vec<Key>> {
    let mut ctr = 0;
    let mut keys = Vec::new();

    while ctr < packets.len() {
        println!("{}/{}", ctr, packets.len());
        let (next_ctr, key) = parse_single(ctr, packets)?;
        ctr = next_ctr;
        keys.push(key);
    }

    // TODO: better error handling
    assert_eq!(ctr, packets.len(), "failed to process all packets");

    Ok(keys)
}