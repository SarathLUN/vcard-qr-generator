fn main() {
    let password = std::env::args().nth(1).unwrap_or_else(|| "admin".to_string());
    let hash = bcrypt::hash(&password, 12).unwrap();
    println!("Password: {}", password);
    println!("Hash: {}", hash);

    // Verify it works
    let verified = bcrypt::verify(&password, &hash).unwrap();
    println!("Verification: {}", if verified { "✓ SUCCESS" } else { "✗ FAILED" });
}
