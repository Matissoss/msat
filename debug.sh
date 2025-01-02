set -e

rm -rf debug

cd admin_dashboard
cargo build --release
cd ../app_server 
cargo build --release 
cd ..

mkdir debug
mv admin_dashboard/target/release/admin_dashboard debug/admin_dashboard 
mv app_server/target/release/app_server debug/app_server 
cp -r admin_dashboard/web debug/web 

echo Finished
