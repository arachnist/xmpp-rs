extern crate minidom;
extern crate sha2;
extern crate sha3;
extern crate blake2;
extern crate base64;

use minidom::Element;

use error::Error;

use disco::{Feature, Identity, Disco, parse_disco};
use data_forms::DataForm;

use self::sha2::{Sha256, Sha512, Digest};
use self::sha3::{Sha3_256, Sha3_512};
use self::blake2::Blake2b;

fn compute_item(field: &str) -> Vec<u8> {
    let mut bytes = field.as_bytes().to_vec();
    bytes.push(0x1f);
    bytes
}

fn compute_items<T, F: Fn(&T) -> Vec<u8>>(things: &[T], separator: u8, encode: F) -> Vec<u8> {
    let mut string: Vec<u8> = vec!();
    let mut accumulator: Vec<Vec<u8>> = vec!();
    for thing in things {
        let bytes = encode(thing);
        accumulator.push(bytes);
    }
    // This works using the expected i;octet collation.
    accumulator.sort();
    for mut bytes in accumulator {
        string.append(&mut bytes);
    }
    string.push(separator);
    string
}

fn compute_features(features: &[Feature]) -> Vec<u8> {
    compute_items(features, 0x1c, |feature| compute_item(&feature.var))
}

fn compute_identities(identities: &[Identity]) -> Vec<u8> {
    compute_items(identities, 0x1c, |identity| {
        let mut bytes = compute_item(&identity.category);
        bytes.append(&mut compute_item(&identity.type_));
        bytes.append(&mut compute_item(&identity.xml_lang));
        bytes.append(&mut compute_item(&identity.name.clone().unwrap_or_default()));
        bytes.push(0x1e);
        bytes
    })
}

fn compute_extensions(extensions: &[DataForm]) -> Vec<u8> {
    compute_items(extensions, 0x1c, |extension| {
        compute_items(&extension.fields, 0x1d, |field| {
            let mut bytes = compute_item(&field.var);
            bytes.append(&mut compute_items(&field.values, 0x1e,
                                            |value| compute_item(value)));
            bytes
        })
    })
}

pub fn compute_disco(disco: &Disco) -> Vec<u8> {
    let features_string = compute_features(&disco.features);
    let identities_string = compute_identities(&disco.identities);
    let extensions_string = compute_extensions(&disco.extensions);

    let mut final_string = vec!();
    final_string.extend(features_string);
    final_string.extend(identities_string);
    final_string.extend(extensions_string);
    final_string
}

pub fn convert_element(root: &Element) -> Result<Vec<u8>, Error> {
    let disco = parse_disco(root)?;
    let final_string = compute_disco(&disco);
    Ok(final_string)
}

// TODO: make algo into an enum.
pub fn hash_ecaps2(data: &[u8], algo: &str) -> String {
    match algo {
        "sha-256" => {
            let mut hasher = Sha256::default();
            hasher.input(data);
            let hash = hasher.result();
            base64::encode(&hash)
        },
        "sha-512" => {
            let mut hasher = Sha512::default();
            hasher.input(data);
            let hash = hasher.result();
            base64::encode(&hash)
        },
        "sha3-256" => {
            let mut hasher = Sha3_256::default();
            hasher.input(data);
            let hash = hasher.result();
            base64::encode(&hash)
        },
        "sha3-512" => {
            let mut hasher = Sha3_512::default();
            hasher.input(data);
            let hash = hasher.result();
            base64::encode(&hash)
        },
        /*
        "blake2b-256" => {
            // TODO: bit length is most likely wrong here!
            let mut hasher = Blake2b::default();
            hasher.input(data);
            let hash = hasher.result();
            base64::encode(&hash)
        },
        "blake2b-512" => {
            // TODO: bit length is most likely wrong here!
            let mut hasher = Blake2b::default();
            hasher.input(data);
            let hash = hasher.result();
            base64::encode(&hash)
        },
        */
        _ => panic!(),
    }
}

#[cfg(test)]
mod tests {
    use minidom::Element;
    use ecaps2;

    #[test]
    fn test_simple() {
        let elem: Element = "<query xmlns='http://jabber.org/protocol/disco#info'><identity category='client' type='pc'/><feature var='http://jabber.org/protocol/disco#info'/></query>".parse().unwrap();
        let ecaps2 = ecaps2::convert_element(&elem).unwrap();
        assert_eq!(ecaps2.len(), 54);
    }

    #[test]
    fn test_xep_ex1() {
        let elem: Element = r#"
<query xmlns="http://jabber.org/protocol/disco#info">
  <identity category="client" name="BombusMod" type="mobile"/>
  <feature var="http://jabber.org/protocol/si"/>
  <feature var="http://jabber.org/protocol/bytestreams"/>
  <feature var="http://jabber.org/protocol/chatstates"/>
  <feature var="http://jabber.org/protocol/disco#info"/>
  <feature var="http://jabber.org/protocol/disco#items"/>
  <feature var="urn:xmpp:ping"/>
  <feature var="jabber:iq:time"/>
  <feature var="jabber:iq:privacy"/>
  <feature var="jabber:iq:version"/>
  <feature var="http://jabber.org/protocol/rosterx"/>
  <feature var="urn:xmpp:time"/>
  <feature var="jabber:x:oob"/>
  <feature var="http://jabber.org/protocol/ibb"/>
  <feature var="http://jabber.org/protocol/si/profile/file-transfer"/>
  <feature var="urn:xmpp:receipts"/>
  <feature var="jabber:iq:roster"/>
  <feature var="jabber:iq:last"/>
</query>
"#.parse().unwrap();
        let expected = vec![104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98,
            101, 114, 46, 111, 114, 103, 47, 112, 114, 111, 116, 111, 99, 111,
            108, 47, 98, 121, 116, 101, 115, 116, 114, 101, 97, 109, 115, 31,
            104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111,
            114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 99, 104,
            97, 116, 115, 116, 97, 116, 101, 115, 31, 104, 116, 116, 112, 58,
            47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114,
            111, 116, 111, 99, 111, 108, 47, 100, 105, 115, 99, 111, 35, 105,
            110, 102, 111, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98,
            101, 114, 46, 111, 114, 103, 47, 112, 114, 111, 116, 111, 99, 111,
            108, 47, 100, 105, 115, 99, 111, 35, 105, 116, 101, 109, 115, 31,
            104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111,
            114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 105, 98,
            98, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114,
            46, 111, 114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47,
            114, 111, 115, 116, 101, 114, 120, 31, 104, 116, 116, 112, 58, 47,
            47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114,
            111, 116, 111, 99, 111, 108, 47, 115, 105, 31, 104, 116, 116, 112,
            58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112,
            114, 111, 116, 111, 99, 111, 108, 47, 115, 105, 47, 112, 114, 111,
            102, 105, 108, 101, 47, 102, 105, 108, 101, 45, 116, 114, 97, 110,
            115, 102, 101, 114, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113,
            58, 108, 97, 115, 116, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113,
            58, 112, 114, 105, 118, 97, 99, 121, 31, 106, 97, 98, 98, 101, 114,
            58, 105, 113, 58, 114, 111, 115, 116, 101, 114, 31, 106, 97, 98,
            98, 101, 114, 58, 105, 113, 58, 116, 105, 109, 101, 31, 106, 97,
            98, 98, 101, 114, 58, 105, 113, 58, 118, 101, 114, 115, 105, 111,
            110, 31, 106, 97, 98, 98, 101, 114, 58, 120, 58, 111, 111, 98, 31,
            117, 114, 110, 58, 120, 109, 112, 112, 58, 112, 105, 110, 103, 31,
            117, 114, 110, 58, 120, 109, 112, 112, 58, 114, 101, 99, 101, 105,
            112, 116, 115, 31, 117, 114, 110, 58, 120, 109, 112, 112, 58, 116,
            105, 109, 101, 31, 28, 99, 108, 105, 101, 110, 116, 31, 109, 111,
            98, 105, 108, 101, 31, 31, 66, 111, 109, 98, 117, 115, 77, 111,
            100, 31, 30, 28, 28];
        let ecaps2 = ecaps2::convert_element(&elem).unwrap();
        assert_eq!(ecaps2.len(), 0x1d9);
        assert_eq!(ecaps2, expected);

        let sha_256 = ecaps2::hash_ecaps2(&ecaps2, "sha-256");
        assert_eq!(sha_256, "kzBZbkqJ3ADrj7v08reD1qcWUwNGHaidNUgD7nHpiw8=");
        let sha3_256 = ecaps2::hash_ecaps2(&ecaps2, "sha3-256");
        assert_eq!(sha3_256, "79mdYAfU9rEdTOcWDO7UEAt6E56SUzk/g6TnqUeuD9Q=");
    }

    #[test]
    fn test_xep_ex2() {
        let elem: Element = r#"
<query xmlns="http://jabber.org/protocol/disco#info">
  <identity category="client" name="Tkabber" type="pc" xml:lang="en"/>
  <identity category="client" name="Ткаббер" type="pc" xml:lang="ru"/>
  <feature var="games:board"/>
  <feature var="http://jabber.org/protocol/activity"/>
  <feature var="http://jabber.org/protocol/activity+notify"/>
  <feature var="http://jabber.org/protocol/bytestreams"/>
  <feature var="http://jabber.org/protocol/chatstates"/>
  <feature var="http://jabber.org/protocol/commands"/>
  <feature var="http://jabber.org/protocol/disco#info"/>
  <feature var="http://jabber.org/protocol/disco#items"/>
  <feature var="http://jabber.org/protocol/evil"/>
  <feature var="http://jabber.org/protocol/feature-neg"/>
  <feature var="http://jabber.org/protocol/geoloc"/>
  <feature var="http://jabber.org/protocol/geoloc+notify"/>
  <feature var="http://jabber.org/protocol/ibb"/>
  <feature var="http://jabber.org/protocol/iqibb"/>
  <feature var="http://jabber.org/protocol/mood"/>
  <feature var="http://jabber.org/protocol/mood+notify"/>
  <feature var="http://jabber.org/protocol/rosterx"/>
  <feature var="http://jabber.org/protocol/si"/>
  <feature var="http://jabber.org/protocol/si/profile/file-transfer"/>
  <feature var="http://jabber.org/protocol/tune"/>
  <feature var="http://www.facebook.com/xmpp/messages"/>
  <feature var="http://www.xmpp.org/extensions/xep-0084.html#ns-metadata+notify"/>
  <feature var="jabber:iq:avatar"/>
  <feature var="jabber:iq:browse"/>
  <feature var="jabber:iq:dtcp"/>
  <feature var="jabber:iq:filexfer"/>
  <feature var="jabber:iq:ibb"/>
  <feature var="jabber:iq:inband"/>
  <feature var="jabber:iq:jidlink"/>
  <feature var="jabber:iq:last"/>
  <feature var="jabber:iq:oob"/>
  <feature var="jabber:iq:privacy"/>
  <feature var="jabber:iq:roster"/>
  <feature var="jabber:iq:time"/>
  <feature var="jabber:iq:version"/>
  <feature var="jabber:x:data"/>
  <feature var="jabber:x:event"/>
  <feature var="jabber:x:oob"/>
  <feature var="urn:xmpp:avatar:metadata+notify"/>
  <feature var="urn:xmpp:ping"/>
  <feature var="urn:xmpp:receipts"/>
  <feature var="urn:xmpp:time"/>
  <x xmlns="jabber:x:data" type="result">
    <field type="hidden" var="FORM_TYPE">
      <value>urn:xmpp:dataforms:softwareinfo</value>
    </field>
    <field var="software">
      <value>Tkabber</value>
    </field>
    <field var="software_version">
      <value>0.11.1-svn-20111216-mod (Tcl/Tk 8.6b2)</value>
    </field>
    <field var="os">
      <value>Windows</value>
    </field>
    <field var="os_version">
      <value>XP</value>
    </field>
  </x>
</query>
"#.parse().unwrap();
        let expected = vec![103, 97, 109, 101, 115, 58, 98, 111, 97, 114, 100,
            31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46,
            111, 114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 97,
            99, 116, 105, 118, 105, 116, 121, 31, 104, 116, 116, 112, 58, 47,
            47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114,
            111, 116, 111, 99, 111, 108, 47, 97, 99, 116, 105, 118, 105, 116,
            121, 43, 110, 111, 116, 105, 102, 121, 31, 104, 116, 116, 112, 58,
            47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114,
            111, 116, 111, 99, 111, 108, 47, 98, 121, 116, 101, 115, 116, 114,
            101, 97, 109, 115, 31, 104, 116, 116, 112, 58,47, 47, 106, 97, 98,
            98, 101, 114, 46, 111, 114, 103, 47, 112, 114, 111, 116, 111, 99,
            111, 108, 47, 99, 104, 97, 116, 115, 116, 97, 116, 101, 115, 31,
            104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111,
            114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 99, 111,
            109, 109, 97, 110, 100, 115, 31,104,116, 116, 112, 58, 47, 47, 106,
            97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114, 111, 116,
            111, 99, 111, 108, 47, 100, 105, 115, 99, 111, 35, 105, 110, 102,
            111, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114,
            46, 111, 114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47,
            100, 105, 115, 99, 111, 35, 105, 116, 101, 109, 115, 31, 104, 116,
            116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103,
            47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 101, 118, 105, 108,
            31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46,
            111, 114, 103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 102,
            101, 97, 116, 117, 114, 101, 45, 110, 101, 103, 31, 104, 116, 116,
            112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47,
            112, 114, 111, 116, 111, 99, 111, 108, 47, 103, 101, 111, 108, 111,
            99, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114,
            46, 111, 114, 103, 47, 112, 114, 111, 116, 111, 99,111, 108, 47,
            103, 101, 111, 108, 111, 99, 43, 110, 111, 116, 105, 102, 121, 31,
            104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111,
            114, 103,47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 105, 98,
            98, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114,
            46, 111, 114, 103, 47, 112, 114, 111,116, 111, 99, 111, 108, 47,
            105, 113, 105, 98, 98, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97,
            98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114, 111, 116,111,
            99, 111, 108, 47, 109, 111, 111, 100, 31, 104, 116, 116, 112, 58,
            47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114,
            111, 116, 111, 99, 111,108, 47, 109, 111, 111, 100, 43, 110, 111,
            116, 105, 102, 121, 31, 104, 116, 116, 112, 58, 47, 47, 106, 97,
            98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114, 111, 116, 111,
            99, 111, 108, 47, 114, 111, 115, 116, 101, 114, 120, 31, 104, 116,
            116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103,
            47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 115, 105, 31, 104,
            116, 116, 112, 58, 47, 47, 106, 97, 98, 98, 101, 114, 46, 111, 114,
            103, 47, 112, 114, 111, 116, 111, 99, 111, 108, 47, 115, 105, 47,
            112, 114, 111, 102, 105, 108, 101, 47, 102, 105, 108, 101, 45, 116,
            114, 97, 110, 115, 102, 101, 114, 31, 104, 116, 116, 112, 58, 47,
            47, 106, 97, 98, 98, 101, 114, 46, 111, 114, 103, 47, 112, 114,
            111, 116, 111, 99, 111, 108, 47, 116, 117, 110, 101, 31, 104, 116,
            116, 112, 58, 47, 47, 119, 119, 119, 46, 102, 97, 99, 101, 98, 111,
            111, 107, 46, 99, 111, 109, 47, 120, 109, 112, 112, 47, 109, 101,
            115, 115, 97, 103, 101, 115, 31, 104, 116, 116, 112, 58, 47, 47,
            119, 119, 119, 46, 120, 109, 112, 112, 46, 111, 114, 103, 47, 101,
            120, 116, 101, 110, 115, 105, 111, 110, 115, 47, 120, 101, 112, 45,
            48, 48, 56, 52, 46, 104, 116, 109, 108, 35, 110, 115, 45, 109, 101,
            116, 97, 100, 97, 116, 97, 43, 110, 111, 116, 105, 102, 121, 31,
            106, 97, 98, 98, 101, 114,58, 105,113, 58, 97, 118, 97, 116, 97,
            114, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113, 58, 98, 114, 111,
            119, 115, 101, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113, 58,
            100, 116, 99, 112, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113, 58,
            102, 105, 108, 101, 120, 102, 101, 114, 31, 106, 97, 98, 98, 101,
            114, 58, 105, 113, 58, 105, 98, 98, 31, 106, 97, 98, 98, 101, 114,
            58, 105, 113, 58, 105, 110, 98, 97, 110, 100, 31, 106, 97, 98, 98,
            101, 114, 58, 105, 113, 58, 106, 105, 100, 108, 105, 110, 107, 31,
            106, 97, 98, 98, 101, 114, 58, 105, 113, 58, 108, 97, 115, 116, 31,
            106, 97, 98, 98, 101, 114, 58, 105, 113, 58, 111, 111, 98, 31, 106,
            97,98, 98, 101, 114, 58, 105, 113, 58, 112, 114, 105, 118, 97, 99,
            121, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113, 58, 114, 111,
            115, 116, 101, 114,31, 106, 97, 98, 98, 101, 114, 58, 105, 113, 58,
            116, 105, 109, 101, 31, 106, 97, 98, 98, 101, 114, 58, 105, 113,
            58, 118, 101, 114, 115, 105, 111, 110, 31, 106, 97, 98, 98, 101,
            114, 58, 120, 58, 100, 97, 116, 97, 31, 106, 97, 98, 98, 101, 114,
            58, 120, 58, 101, 118, 101, 110, 116, 31, 106, 97, 98, 98, 101,
            114, 58, 120, 58, 111, 111, 98, 31, 117, 114, 110, 58, 120, 109,
            112, 112, 58, 97, 118, 97, 116, 97, 114, 58, 109, 101, 116, 97,
            100, 97, 116, 97, 43, 110, 111, 116, 105, 102, 121,31, 117, 114,
            110, 58, 120, 109, 112, 112, 58, 112, 105, 110, 103, 31, 117, 114,
            110, 58, 120, 109, 112, 112, 58, 114, 101, 99, 101, 105, 112, 116,
            115, 31, 117, 114, 110, 58, 120, 109, 112, 112, 58, 116, 105, 109,
            101, 31, 28, 99, 108, 105, 101, 110, 116, 31, 112, 99, 31, 101,
            110, 31, 84, 107, 97, 98, 98, 101, 114,31, 30, 99, 108, 105, 101,
            110, 116, 31, 112, 99, 31, 114, 117, 31, 208, 162, 208, 186, 208,
            176, 208, 177, 208, 177, 208, 181, 209, 128, 31, 30, 28, 70, 79,
            82, 77, 95, 84, 89, 80, 69, 31, 117, 114, 110, 58, 120, 109, 112,
            112, 58, 100, 97, 116, 97, 102, 111, 114, 109, 115, 58, 115, 111,
            102, 116, 119, 97, 114, 101,105, 110, 102, 111, 31, 30, 111, 115,
            31, 87, 105, 110, 100, 111, 119, 115, 31, 30, 111, 115, 95, 118,
            101, 114, 115, 105, 111, 110, 31, 88, 80, 31, 30, 115, 111, 102,
            116, 119, 97, 114, 101, 31, 84, 107, 97, 98, 98, 101, 114, 31, 30,
            115, 111, 102, 116, 119, 97, 114, 101, 95, 118, 101, 114, 115, 105,
            111, 110, 31, 48, 46, 49, 49, 46, 49, 45, 115, 118, 110, 45, 50,
            48, 49, 49, 49, 50, 49, 54, 45, 109, 111, 100, 32, 40, 84, 99, 108,
            47, 84, 107, 32, 56, 46,54, 98, 50, 41, 31, 30, 29, 28];
        let ecaps2 = ecaps2::convert_element(&elem).unwrap();
        assert_eq!(ecaps2.len(), 0x543);
        assert_eq!(ecaps2, expected);

        let sha_256 = ecaps2::hash_ecaps2(&ecaps2, "sha-256");
        assert_eq!(sha_256, "u79ZroNJbdSWhdSp311mddz44oHHPsEBntQ5b1jqBSY=");
        let sha3_256 = ecaps2::hash_ecaps2(&ecaps2, "sha3-256");
        assert_eq!(sha3_256, "XpUJzLAc93258sMECZ3FJpebkzuyNXDzRNwQog8eycg=");
    }
}
