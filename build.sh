# Clear Build directory
rm -rf build
# Compile Admin Dashboard
cd http_server
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-gnu 
# Compile server
cd ../server 
cargo build --release --target x86_64-unknown-linux-gnu
cargo build --release --target x86_64-pc-windows-gnu 
# Move out of server dir
cd ..
# Make build directories
mkdir build
mkdir build/winx86_64
mkdir build/linx86_64
# Move files to build directories
mv http_server/target/x86_64-unknown-linux-gnu/release/mhs-bin build/linx86_64/mhs-bin 
mv http_server/target/x86_64-pc-windows-gnu/release/mhs-bin.exe build/winx86_64/mhs-bin.exe 
mv server/target/x86_64-unknown-linux-gnu/release/server build/linx86_64/server 
mv server/target/x86_64-pc-windows-gnu/release/server.exe build/winx86_64/server.exe 
# Copy admin dashboard to build dirs
cp -r http_server/web build/linx86_64/web 
cp -r http_server/web build/winx86_64/web

# Make directory for compressed binaries
mkdir build/release 
tar -czvf build/release/winx86_64.tar.gz build/winx86_64/
tar -czvf build/release/linx86_64.tar.gz build/linx86_64/

echo Finished building project
