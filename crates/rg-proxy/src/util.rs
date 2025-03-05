use bytes::{Bytes, BytesMut};

pub fn remove_headers(content: &[u8], keyword: &str) -> Bytes {
    let mut new_content = BytesMut::new();

    let mut index = 0;

    while let Some(line_end) = content[index..].iter().position(|&b| b == b'\n') {
        let line_start = index;
        index += line_end + 1;
        if let Ok(header_line) = std::str::from_utf8(&content[line_start..index]) {
            if !header_line.to_uppercase().contains(keyword) {
                new_content.extend_from_slice(header_line.as_bytes());
            }
        }
    }

    if index < content.len() {
        new_content.extend_from_slice(&content[index..]);
    }
    new_content.freeze()
}

#[cfg(test)]
mod test {
    use bytes::Bytes;

    #[test]
    fn test_remove_header() {
        let content = b"CONNECT www.baidu.com:443 HTTP/1.1\r\nHost: www.baidu.com:443\r\nProxy-Connection: keep-alive\r\nProxy-Authorization: Basic dXNlcm5hbWU6cGFzc3dvcmQ=\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)\r\n\r\n";

        let new_content = super::remove_headers(content, "PROXY-");
        let expected = Bytes::from_static(b"CONNECT www.baidu.com:443 HTTP/1.1\r\nHost: www.baidu.com:443\r\nUser-Agent: Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko)\r\n\r\n");
        assert_eq!(new_content, expected);
        println!("{:?}", new_content);
    }

    #[test]
    fn test_remove_header1() {
        let content = b"POST http://ticket.yes24.com:80/Pages/English/Sale/FnPerfSaleProcess.aspx?IdPerf=50429 HTTP/1.1\r\nuser-agent: Mozilla/5.0 (Macintosh; U; Intel Mac OS X 10_6_3; en-US) AppleWebKit/534.3 (KHTML, like Gecko) Chrome/6.0.456.0 Safari/534.3\r\naccept-language: zh-CN,zh;q=0.9\r\norigin: http://ticket.yes24.com\r\naccept: text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7\r\nreferer: http://ticket.yes24.com/Pages/English/Perf/FnPerfDeail.aspx?IdPerf=50429\r\ncontent-type: application/x-www-form-urlencoded\r\ncookie: ASP.NET_SessionId=nrp0zdmgnqbe4205h15quggh;YesTicketForeign=UserNO=23,94,62,201,22,174,58,222,1,69,132,99,8,237,157,117,238,211,210,204,113,218,124,44&UserName=91,40,65,219,84,140,249,105,41,66,117,92,125,171,152,205&Email=2,91,64,14,160,54,245,21,3,43,32,26,153,56,59,45,187,205,3,30,172,137,116,119,63,228,131,112,8,45,175,44&UserIdentiNumber=248,177,66,242,215,38,182,92,138,56,85,27,65,146,176,1&Phone=245,54,43,158,39,91,228,116&Mobile=245,54,43,158,39,91,228,116&IdType=60,171,81,93,255,57,213,127&MobileType=195,31,27,161,165,198,213,175&ServiceCookie=120,241,37,146,107,141,250,187,71,210,43,192,52,235,30,216,220,65,26,232,180,28,74,111,145,145,252,51,223,51,30,26,121,79,126,207,96,174,245,237,174,171,79,136,36,126,214,22;NetFunnel_ID=5002%3A200%3Akey%3D0925C611B790DDAAD4256BAA60B820C39CC0A48592DDF742BC295C0B51F74C4A08B31BA68F3A6EE8E15AEEA19610CE9B177ACEB9B71EEF440930BA7C5EC06FE5310973558EDF16341B671FD594E7C7EA639F976594360FFC363611677B4CE3F7FD0F19F38CF52E919F547DED97725EA6312C302C30%26nwait%3D0%26nnext%3D0%26tps%3D0.000000%26ttl%3D0%26ip%3Dtkwait.yes24.com%26port%3D443\r\naccept-encoding: gzip, br\r\nhost: ticket.yes24.com\r\ncontent-length: 2105\r\npragma: no-cache\r\ncache-control: no-cache\r\nConnection: close\r\nProxy-Authorization: Basic dWltcWwzcWk6bHE4Z3l0MjY=\r\n\r\n__VIEWSTATE=%2FwEPDwUKMTExNzQzODUxNQ9kFgICAQ9kFhgCAQ8WAh4Dc3JjBUtodHRwOi8vdGtmaWxlLnllczI0LmNvbS91cGxvYWQyL1BlcmZCbG9nLzIwMjQwNy8yMDI0MDcyMi8yMDI0MDcyMi01MDQyOS5qcGdkAgMPFgIeBFRleHQFTFNFUFVMVFVSQSDigJhDZWxlYnJhdGluZyBMaWZlIFRocm91Z2ggRGVhdGjigJkgLSBGQVJFV0VMTCBUT1VSIDIwMjQgSU4gU0VPVUxkAgQPFgIfAWVkAgUPFgIfAQUMTGl2ZSBDb25jZXJ0ZAIGDxYCHwEFC0F1ZyA1LCAyMDI0ZAIHDxYCHwEFFVlFUzI0IFdBTkRFUkxPQ0ggSEFMTGQCCA8WAh8BBRA4IHllYXJzIGFuZCBvdmVyZAIJDxYCHwEFGmdsb2JhbF95ZXN0aWNrZXRAeWVzMjQuY29tZAIKDxYCHwEFCzEwMCBtaW51dGVzZAILDxYEHgRocmVmBTxqYXZhc2NyaXB0OmdvT3BlblBlcmZGTlNhbGVNc2coJ04yMDI0MDcwMTExMTIzM2MzZCcsIDUwNDI5KTseB1Zpc2libGVnZAIODxYCHwEFKjxsaT5TdGFuZGluZyA6IDxzcGFuPiBXODgsMDAwIDwvc3Bhbj48L2xpPmQCDw8WAh8BBbUG4oC7IFBsZWFzZSBtYWtlIHN1cmUgeW91IGFyZSBzaWduZWQgdXAgYW5kIGxvZ2dlZCBpbiBwcmlvciB0byB0aGUgcHVyY2hhc2UuPGJyPuKAuyBDcmVkaXQgY2FyZChFeGNlcHQgZm9yIEFtZXJpY2FuIEV4cHJlc3MgY2FyZHMgaXNzdWVkIG91dHNpZGUgb2YgUmVwdWJsaWMgb2YgS29yZWEgYW5kIENoaW5hIFVuaW9uIFBheSBjYXJkcykgYW5kIFBheVBhbCBpcyBhdmFpbGFibGUgZm9yIHBheW1lbnQuPGJyPuKAuyBQbGVhc2Ugbm90ZSB0aGF0IHNvbWUgb2YgcGF5bWVudCBtZXRob2RzIG1heSBub3QgYmUgYWJsZSB0byB1c2Ugd2hlbiB5b3UgcGxhY2UgeW91ciBvcmRlci48cCBzdHlsZT0idGV4dC1hbGlnbjogbGVmdDsiPjxpbWcgc3JjPSJodHRwOi8vdGtmaWxlLnllczI0LmNvbS9VcGxvYWQyL0JvYXJkLzIwMjQwNy8yMDI0MDcyMi8yNDA3MjIxNl8wMS5qcGciIGNsYXNzPSJ0eGMtaW1hZ2UiIHN0eWxlPSJjbGVhcjpub25lO2Zsb2F0Om5vbmU7IiAvPjwvcD48cCBzdHlsZT0idGV4dC1hbGlnbjogbGVmdDsiPjxpbWcgc3JjPSJodHRwOi8vdGtmaWxlLnllczI0LmNvbS9VcGxvYWQyL0JvYXJkLzIwMjQwNy8yMDI0MDcyMi8yNDA3MjIxNl8wMi5qcGciIGNsYXNzPSJ0eGMtaW1hZ2UiIHN0eWxlPSJjbGVhcjpub25lO2Zsb2F0Om5vbmU7IiAvPjwvcD48cCBzdHlsZT0idGV4dC1hbGlnbjogbGVmdDsiPjxpbWcgc3JjPSJodHRwOi8vdGtmaWxlLnllczI0LmNvbS9VcGxvYWQyL0JvYXJkLzIwMjQwNy8yMDI0MDcyMi8yNDA3MjIxNl8wMy5qcGciIGNsYXNzPSJ0eGMtaW1hZ2UiIHN0eWxlPSJjbGVhcjpub25lO2Zsb2F0Om5vbmU7IiAvPjwvcD5kZJgQJZYZyqUEfSUutXSlpqeQN8r4&__VIEWSTATEGENERATOR=B08D154D&netfunnel_key=0925C611B790DDAAD4256BAA60B820C39CC0A48592DDF742BC295C0B51F74C4A08B31BA68F3A6EE8E15AEEA19610CE9B177ACEB9B71EEF440930BA7C5EC06FE5310973558EDF16341B671FD594E7C7EA639F976594360FFC363611677B4CE3F7FD0F19F38CF52E919F547DED97725EA6312C302C30";
        let new_content = super::remove_headers(content, "PROXY-");
        println!("{:?}", new_content);
    }
}
