pub mod auth;
pub mod message;

const PORT_START: u32 = 51000;
const PORT_END: u32 = 63000;

pub fn ip_to_ip_port(ip: &str) -> Option<String> {
    let parts = ip.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return None;
    }
    let ip_num = parts
        .iter()
        .map(|x| x.parse::<u32>().unwrap())
        .fold(0, |acc, x| acc * 256 + x);
    let port_range = PORT_END - PORT_START;
    let port = PORT_START + ip_num % port_range;
    format!("{}:{}", ip, port).into()
}

#[cfg(test)]
mod test {
    use crate::ip_to_ip_port;

    #[test]
    fn test_ip_to_ip_port() {
        println!("{:?}", ip_to_ip_port("175.197.89.2"));
    }
}
