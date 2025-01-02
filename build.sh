set -e 

# Clear Build directory
rm -rf build
# Compile Admin Dashboard
cd admin_dashboard

cargo build --release --target x86_64-unknown-linux-gnu &
cargo build --release --target x86_64-unknown-linux-musl &
cargo build --release --target x86_64-pc-windows-gnu &
wait

# Compile server
cd ../app_server

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
mv admin_dashboard/target/x86_64-unknown-linux-gnu/release/admin_dashboard build/linx86_64-libc/admin_dashboard
mv admin_dashboard/target/x86_64-unknown-linux-musl/release/admin_dashboard build/linx86_64-musl/admin_dashboard
mv admin_dashboard/target/x86_64-pc-windows-gnu/release/admin_dashboard.exe build/winx86_64/admin_dashboard.exe
# Now Application Server
mv app_server/target/x86_64-unknown-linux-gnu/release/app_server build/linx86_64-libc/app_server 
mv app_server/target/x86_64-unknown-linux-musl/release/app_server build/linx86_64-musl/app_server 
mv app_server/target/x86_64-pc-windows-gnu/release/app_server.exe build/winx86_64/app_server.exe 
# Copy admin dashboard to build dirs
cp -r admin_dashboard/web build/linx86_64-libc/web
cp -r admin_dashboard/web build/linx86_64-musl/web
cp -r admin_dashboard/web build/winx86_64/web

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
