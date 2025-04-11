[![progress-banner](https://backend.codecrafters.io/progress/dns-server/e1ce1151-13b2-4510-b7d1-e611176e6810)](https://app.codecrafters.io/users/jmontroy90?r=2qF)

This is my Rust solution for the CodeCrafter's ["Build Your Own DNS server" Challenge](https://app.codecrafters.io/courses/dns-server/overview).

tl;dr - this was a real challenge with:
- Bit-packing the header, bit shifts and all that.
- Parsing and reading different formats of low-level bytes.
- Rust is a nitpicky language.
- The QNAME pointer requires the full packet to resolve, but my initial implementation consumed a `BytesMut` via like `buf.get_u16()` a lot, such that pointers couldn't be resolved in line with the rest of the code. Thinking of a way to defer the pointer resolve was tricky, and I don't really think it's an amazing solution. Much better would be to use a `Cursor` that allows you to move around the buffer as you go. Maybe in the future.