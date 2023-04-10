# Byteskill
*byte skill*

A Rust-based clone of the **Web Assessment Tool**, used in the Department of Engineering and Computer Science at Victoria University of Wellington.

This tool allows users to complete languages in the Java programming language,
and have them evaluated on the server. It allows students to be quizzed on their Java skill
without requiring manual human marking or requiring students to set up a Java
integrated development environment on their computer.

### Infrastructure
Byteskill stores the provided user code in a temporary folder on the server (provided by the `tempdir`
crate which uses system-provided primitives), which is then mounted to an Ubuntu Java 17
Docker image, which executes the code and returns the result.
