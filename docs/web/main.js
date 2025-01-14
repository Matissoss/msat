function $(html){
	return document.getElementById(html);
}

const NAVBAR  = $("NAV");
const CONTENT = $("ROOT");

function main(){
	for (let i = 0; i < NAVBAR.childElementCount; i++){
		let child = NAVBAR.children[i];
		if (child.nodeName === "A"){
			child.addEventListener('click', () => {
				switch_mode(child.getAttribute('href'))
			})
		}
	}
}

function switch_mode(hash){
	switch (hash){
		case "#home":
			alert("Home!");
			break;
		case "#docs":
			alert("Docs!");
			break;
		case "#setup":
			alert("Setup!");
			break;
		default:
			alert("Home!");
			break;
	}
}

main();
