use oauth2::{PkceCodeChallenge, PkceCodeVerifier};
use std::io;
use xal;

fn main() -> io::Result<()> {
    let mut authenticator = xal::authenticator::XalAuthenticator::default();

    let mut authorization_code = String::new();
    let stdin = io::stdin(); // We get `Stdin` here.
    let _ = stdin.read_line(&mut authorization_code)?;

    let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();

    println!("{:?} {:?}", challenge, verifier);
    println!("{:?}", challenge.as_str());
    println!("{:?}", verifier.secret());

    Ok(())
}
