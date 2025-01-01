set -e 

# Clear Build directory
rm -rf build
# Compile Admin Dashboard
cd http_server

cargo build --release --target x86_64-unknown-linux-gnu &
cargo build --release --target x86_64-unknown-linux-musl &
cargo build --release --target x86_64-pc-windows-gnu &
wait

# Compile server
cd ../server 

cargo build --release --target x86_64-unknown-linux-gnu &
cargo build --release --target x86_64-unknown-linux-musl &
cargo build --release --target x86_64-pc-windows-gnu &
wait

# Move out of server dir
cd ..
# Make build directories - native means that file is compiled for your CPU and OS architecture
mkdir build
mkdir build/winx86_64
mkdir build/linx86_64-libc
mkdir build/linx86_64-musl
# Move files to build directories
mv http_server/target/x86_64-unknown-linux-gnu/release/mhs-bin build/linx86_64-libc/mhs-bin 
mv http_server/target/x86_64-unknown-linux-musl/release/mhs-bin build/linx86_64-musl/mhs-bin
mv http_server/target/x86_64-pc-windows-gnu/release/mhs-bin.exe build/winx86_64/mhs-bin.exe 
# Now Application Server
mv server/target/x86_64-unknown-linux-gnu/release/server build/linx86_64-libc/server 
mv server/target/x86_64-unknown-linux-musl/release/server build/linx86_64-musl/server 
mv server/target/x86_64-pc-windows-gnu/release/server.exe build/winx86_64/server.exe 
# Copy admin dashboard to build dirs
cp -r http_server/web build/linx86_64-libc/web 
cp -r http_server/web build/linx86_64-musl/web 
cp -r http_server/web build/winx86_64/web

# Now copy setup/docs files to build dirs 
cp -r docs build/winx86_64/docs
cp -r docs build/linx86_64-musl/docs
cp -r docs build/linx86_64-libc/docs

# Make directory for compressed binaries
mkdir build/release

tar -czvf build/release/winx86_64.tar.gz build/winx86_64 &
tar -czvf build/release/linx86_64-musl.tar.gz build/linx86_64-musl &
tar -czvf build/release/linx86_64-libc.tar.gz build/linx86_64-libc &
wait

echo Finished building project
