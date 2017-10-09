use std::char;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind, Result};

pub struct Mount {
    source: String,
    dest: String,
    fs: String,
    options: String,
    dump: String,
    pass: String,
}

impl Mount {
    fn parse_value(value: &str) -> Result<String> {
        let mut ret = String::new();

        let mut chars = value.chars();
        while let Some(c) = chars.next() {
            match c {
                '\\' => {
                    let mut code = 0;
                    for _i in 0..3 {
                        if let Some(num) = chars.next() {
                            code *= 8;
                            code += u32::from_str_radix(&num.to_string(), 8).map_err(|err| {
                                Error::new(ErrorKind::Other, err)
                            })?;
                        } else {
                            return Err(Error::new(ErrorKind::Other, "truncated octal code"));
                        }
                    }
                    ret.push(char::from_u32(code).ok_or(Error::new(ErrorKind::Other, "invalid octal code"))?);
                },
                _ => {
                    ret.push(c);
                }
            }
        }

        Ok(ret)
    }

    fn parse_line(line: &str) -> Result<Mount> {
        let mut parts = line.split(' ');

        let source = parts.next().ok_or(Error::new(ErrorKind::Other, "Missing source"))?;
        let dest = parts.next().ok_or(Error::new(ErrorKind::Other, "Missing dest"))?;
        let fs = parts.next().ok_or(Error::new(ErrorKind::Other, "Missing fs"))?;
        let options = parts.next().ok_or(Error::new(ErrorKind::Other, "Missing options"))?;
        let dump = parts.next().ok_or(Error::new(ErrorKind::Other, "Missing dump"))?;
        let pass = parts.next().ok_or(Error::new(ErrorKind::Other, "Missing pass"))?;

        Ok(Mount {
            source: Self::parse_value(&source)?,
            dest: Self::parse_value(&dest)?,
            fs: Self::parse_value(&fs)?,
            options: Self::parse_value(&options)?,
            dump: Self::parse_value(&dump)?,
            pass: Self::parse_value(&pass)?
        })
    }

    pub fn all() -> Result<Vec<Mount>> {
        let mut ret = Vec::new();

        let file = BufReader::new(File::open("/proc/self/mounts")?);
        for line_res in file.lines() {
            let line = line_res?;
            ret.push(Self::parse_line(&line)?);
        }

        Ok(ret)
    }
}
