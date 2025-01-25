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
	switch_mode(window.location.hash);
}

function home(){
	CONTENT.innerHTML = `
		<br>
		<br>
		<br>
		<h1>msat</h1>
		<h2>mateus' school administration tool</h2>
		<button onclick='window.open("https://github.com/Matissoss/msat")'>
			Star repo on github</button>
		<hr>
		<h2>Key Features</h2>
		<div class='container'>
			<div class='column'>
				<div class='column'>
					<h3>Efficient</h3>
					<p>
					<strong>msat</strong> was made to be <strong>fast</strong>, 
					<strong>lightweight</strong> and reliable. <br>
					<strong>msat</strong> is fully <i>asynchronous</i> and <i>multithreaded</i>.
					</p>
				</div>
				<br>
				<div class='column'>
					<h3>Open-source</h3>
					<p>
					<strong>msat</strong> is 100% Free and open-source licensed under 
					liberal X11 (MIT) license. 
					This allows <strong>you</strong> to change it how you'd like it.
					</p>
				</div>
				<br>
			</div>
		</div>
		`
}

function switch_mode(hash){
	switch (hash){
		case "#home":
			home();
			break;
		case "#setup":
			window.open("https://github.com/Matissoss/msat/tree/main/setup.md")
			break;
		default:
			home();
			break;
	}
	window.location.hash = hash;
}

main();
