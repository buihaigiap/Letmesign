use openssl::pkcs12::Pkcs12;
use std::fs;

fn main() {
    let data = fs::read("/home/giap/Desktop/test_certificate.p12").unwrap();
    let pkcs12 = Pkcs12::from_der(&data).unwrap();
    
    println!("Testing parse()...");
    match pkcs12.parse("123456") {
        Ok(parsed) => {
            println!("✅ parse() SUCCESS!");
            println!("Certificate: {:?}", parsed.cert.subject_name());
        }
        Err(e) => {
            println!("❌ parse() FAILED: {:?}", e);
        }
    }
    
    println!("\nTesting parse2()...");
    let pkcs12_2 = Pkcs12::from_der(&data).unwrap();
    match pkcs12_2.parse2("123456") {
        Ok(parsed) => {
            println!("✅ parse2() SUCCESS!");
            println!("Certificate exists: {}", parsed.cert.is_some());
        }
        Err(e) => {
            println!("❌ parse2() FAILED: {:?}", e);
        }
    }
}
