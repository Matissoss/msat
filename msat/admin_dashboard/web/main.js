// Util functions
function $(html){
	return document.getElementById(html);
}

function set_cookie(name, value, days) {
  let expires = "";
  if (days) {
    const date = new Date();
    date.setTime(date.getTime() + (days * 24 * 60 * 60 * 1000));
    expires = "; expires=" + date.toUTCString();
  }
  document.cookie = name + "=" + encodeURIComponent(value) + expires + "; path=/";
}

function get_cookie(name) {
  const cookies = document.cookie.split('; ');
  for (let cookie of cookies) {
    const [key, value] = cookie.split('=');
    if (key === name) {
      return decodeURIComponent(value);
    }
  }
  return null;
}

function delete_cookie(name) {
  set_cookie(name, "", -1);
}

// consts
const ROOT = $("ROOT");
const NAVM = $("NAVM");

function main(){
	refresh(window.location.hash);
	for (let i=0; i<NAVM.childElementCount; i++){
		let link = NAVM.children[i];
		link.addEventListener('click', () => {
			refresh(link.getAttribute('href'));
		});
	}
}

function refresh(hash){
}

main();
