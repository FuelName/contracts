library;

use std::string::String;
use std::bytes::Bytes;
use std::primitive_conversions::u64::*;
use std::logging::*;


const MIN_DOMAIN_PART_LENGTH: u64 = 1;
const MIN_DOMAIN_LENGTH: u64 = 3;
const MAX_DOMAIN_LENGTH: u64 = 64;

const DASH_SYMBOL_ASCII: u8 = 45;
const DOT_SYMBOL_ASCII: u8 = 46;

fn convert_num_to_ascii_bytes(num: u64) -> Bytes {
    let mut bytes = Bytes::new();
    let mut n = num;
    if n == 0 {
        bytes.push(48);
        return bytes;
    }
    while n != 0 {
        // 48 - is an ASCII offset for digits
        bytes.push(((n % 10) + 48).try_as_u8().unwrap());
        n /= 10;
    }
    let mut reversed_bytes = Bytes::with_capacity(bytes.len());
    while !bytes.is_empty() {
        reversed_bytes.push(bytes.pop().unwrap());
    }
    return reversed_bytes;
}

fn check_domain_len(domain: String) -> bool {
    let length = domain.as_bytes().len();
    length >= MIN_DOMAIN_LENGTH && length <= MAX_DOMAIN_LENGTH
}

fn check_domain_part_len(domain_part: String) -> bool {
    let length = domain_part.as_bytes().len();
    length >= MIN_DOMAIN_PART_LENGTH && length <= MAX_DOMAIN_LENGTH
}

fn symbol_is_dash(symbol: u8) -> bool {
    symbol == DASH_SYMBOL_ASCII
}

fn symbol_is_dot(symbol: u8) -> bool {
    symbol == DOT_SYMBOL_ASCII
}

fn check_domain_symbol(symbol: u8, allow_dots: bool) -> bool {
    let is_latin_letter = symbol >= 97 && symbol <= 122; // 'a' - 'z'
    let is_digit = symbol >= 48 && symbol <= 57; // '0' - '9'
    let is_dash = symbol_is_dash(symbol);
    let is_allowed_dot = allow_dots && symbol_is_dot(symbol);
    is_latin_letter || is_digit || is_dash || is_allowed_dot
}

fn check_domain_symbols(domain: String, allow_dots: bool) -> bool {
    let bytes = domain.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let symbol = bytes.get(i).unwrap();
        if (!check_domain_symbol(symbol, allow_dots)) {
            return false;
        }
        let is_boundary_symbol = i == 0 || i == bytes.len() - 1;
        if (is_boundary_symbol) {
            if (symbol_is_dash(symbol) || symbol_is_dot(symbol)) {
                return false;
            }
        }
        i = i + 1;
    }
    true
}

pub fn domain_is_allowed(domain: String) -> bool {
    check_domain_len(domain) && check_domain_symbols(domain, true)
}

pub fn domain_part_is_allowed(domain_part: String) -> bool {
    check_domain_part_len(domain_part) && check_domain_symbols(domain_part, false)
}

fn push_bytes(ref mut a: Bytes, b: Bytes) {
    let mut i = 0;
    while i < b.len() {
        a.push(b.get(i).unwrap());
        i = i + 1;
    }
}

fn push_bytes_replace(ref mut a: Bytes, b: Bytes) {
    let mut i = 0;
    let dot_replacement = String::from_ascii_str("%2E").as_bytes();
    let dash_replacement = String::from_ascii_str("%2D").as_bytes();
    while i < b.len() {
        let element = b.get(i).unwrap();
        if element == DOT_SYMBOL_ASCII {
            push_bytes(a, dot_replacement);
        } else if element == DASH_SYMBOL_ASCII {
            push_bytes(a, dash_replacement);
        } else {
            a.push(element);
        }
        i = i + 1;
    }
}

// See https://forum.fuel.network/t/how-to-concatenate-strings-in-sway/4348
pub fn build_domain_name(child: String, parent: String) -> String {
    let mut result = Bytes::new();
    push_bytes(result, child.as_bytes());
    push_bytes(result, String::from_ascii_str(".").as_bytes());
    push_bytes(result, parent.as_bytes());
    String::from_ascii(result)
}

pub fn build_token_uri(domain_name: String) -> String {
    let mut result = Bytes::new();
    push_bytes(result, String::from_ascii_str("https://prod.api.fuelet.app/testnet/fuelname/metadata/").as_bytes());
    push_bytes_replace(result, domain_name.as_bytes());
    String::from_ascii(result)
}

pub fn build_domain_hash_base(domain: String, gen: u64) -> String {
    let mut result = Bytes::new();
    push_bytes(result, domain.as_bytes());
    push_bytes(result, String::from_ascii_str("#").as_bytes());
    push_bytes(result, convert_num_to_ascii_bytes(gen));
    String::from_ascii(result)
}

// Tests
#[test]
fn test_valid_domains_are_allowed() {
    assert(domain_is_allowed(String::from_ascii_str("abcdefghijklmnopqrstuvwxyz")));
    assert(domain_is_allowed(String::from_ascii_str("0123456789")));
    assert(domain_is_allowed(String::from_ascii_str("abcdefghijklmnopqrstuvwxyz---0123456789")));
    assert(domain_is_allowed(String::from_ascii_str("a-b-c")));
    assert(domain_is_allowed(String::from_ascii_str("1-2-3")));
    assert(domain_is_allowed(String::from_ascii_str("abc"))); // min len
    assert(domain_is_allowed(String::from_ascii_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"))); // max len
}
#[test]
fn test_short_domains_are_not_allowed() {
    assert(!domain_is_allowed(String::from_ascii_str("a")));
    assert(!domain_is_allowed(String::from_ascii_str("ab")));
}
#[test]
fn test_long_domains_are_not_allowed() {
    assert(!domain_is_allowed(String::from_ascii_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
    assert(!domain_is_allowed(String::from_ascii_str("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")));
}
#[test]
fn test_non_latin_domains_are_not_allowed() {
    assert(!domain_is_allowed(String::from_ascii_str("a_a_a")));
    assert(!domain_is_allowed(String::from_ascii_str("яяяяя")));
}
#[test]
fn test_uppercase_domains_are_not_allowed() {
    assert(!domain_is_allowed(String::from_ascii_str("AAAAA")));
    assert(!domain_is_allowed(String::from_ascii_str("aaaaA")));
    assert(!domain_is_allowed(String::from_ascii_str("Aaaaa")));
}
#[test]
fn test_starting_or_ending_with_dash_or_dot_domains_are_not_allowed() {
    assert(!domain_is_allowed(String::from_ascii_str("-aaaa")));
    assert(!domain_is_allowed(String::from_ascii_str("aaaa-")));
    assert(!domain_is_allowed(String::from_ascii_str("-aaa-")));
    assert(!domain_is_allowed(String::from_ascii_str(".aaaa.")));
    assert(!domain_is_allowed(String::from_ascii_str("aaaa.")));
    assert(!domain_is_allowed(String::from_ascii_str("-aaa.")));
    assert(!domain_is_allowed(String::from_ascii_str(".aaa-")));
}
#[test]
fn test_short_domain_parts() {
    assert(domain_part_is_allowed(String::from_ascii_str("a")));
    assert(domain_part_is_allowed(String::from_ascii_str("aa")));
    assert(domain_part_is_allowed(String::from_ascii_str("aaa")));
}
#[test]
fn test_dots_are_not_allowed_in_domain_parts() {
    assert(!domain_part_is_allowed(String::from_ascii_str(".")));
    assert(!domain_part_is_allowed(String::from_ascii_str("a.a")));
    assert(!domain_part_is_allowed(String::from_ascii_str("aaaaaaa.a")));
}
#[test]
fn test_build_domain_name() {
    assert(build_domain_name(String::from_ascii_str("a"), String::from_ascii_str("b")) == String::from_ascii_str("a.b"));
    assert(build_domain_name(String::from_ascii_str("abcdef"), String::from_ascii_str("123456")) == String::from_ascii_str("abcdef.123456"));
    assert(build_domain_name(String::from_ascii_str("fuelet"), String::from_ascii_str("fuel")) == String::from_ascii_str("fuelet.fuel"));
}
#[test]
fn test_build_token_uri() {
    assert(build_token_uri(String::from_ascii_str("domain")) == String::from_ascii_str("https://prod.api.fuelet.app/testnet/fuelname/metadata/domain"));
    assert(build_token_uri(String::from_ascii_str("dom-ain.fuel")) == String::from_ascii_str("https://prod.api.fuelet.app/testnet/fuelname/metadata/dom%2Dain%2Efuel"));
    assert(build_token_uri(String::from_ascii_str("one.two.three.fuel")) == String::from_ascii_str("https://prod.api.fuelet.app/testnet/fuelname/metadata/one%2Etwo%2Ethree%2Efuel"));
}
#[test]
fn test_build_domain_hash_base() {
    assert(build_domain_hash_base(String::from_ascii_str("domain"), 0) == String::from_ascii_str("domain#0"));
    assert(build_domain_hash_base(String::from_ascii_str("domain"), 1) == String::from_ascii_str("domain#1"));
    assert(build_domain_hash_base(String::from_ascii_str("domain.fuel"), 50000000000) == String::from_ascii_str("domain.fuel#50000000000"));
    assert(build_domain_hash_base(String::from_ascii_str("one.two.three.fuel"), 123456789) == String::from_ascii_str("one.two.three.fuel#123456789"));
}
