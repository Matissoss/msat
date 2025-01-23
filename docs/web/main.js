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
		<button class="button-gith" onclick='window.open("https://github.com/Matissoss/msat")'>
			Star repo on github</button>
		<hr>
		<h2>Key Features</h2>
		<div class='container'>
			<div class='column'>
				<div class='column'>
					<h3>User-friendly Documentation</h3>
					<p>
					<strong>msat</strong> has friendly documentation for both:
					developers and server administrators
					</p>
				</div>
				<br>
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
				<div class='column'>
					<h3>Cross-platform</h3>
					<p>
					<strong>msat</strong> client is availiable on Android, iOS and Desktop platforms
					</p>
				</div>
			</div>
		</div>
		`
}

function switch_mode(hash){
	switch (hash){
		case "#home":
			home();
			break;
		case "#docs":
			alert("Not Done!");
			break;
		case "#setup":
			CONTENT.innerHTML = 
			`
			<br>
			<br>
			<br>
			<h1>Setup</h1>
			<h3>This will lead you through installation and setup process</h3>
			<hr>
			<h2>1. Download Executables</h2>
			<p> First, you will need to install <i>msat</i>. <i>msat</i> executables can be found 
			in table below:
			</p>
			<table>
				<tr style='font-weight: bolder;'>
					<td>
						Platform and CPU architecture
					</td>
					<td>
						Link
					</td>
				</tr>
				<tr>
					<td>
						Windows x86 64-bit
					</td>
					<td>
						<a href="https://github.com/Matissoss/msat/releases">Link</a>
					</td>
				</tr>
				<tr>
					<td>
						Linux x86 64-bit libc
					</td>
					<td>
						<a href="https://github.com/Matissoss/msat/releases">Link</a>
					</td>
				</tr>
				<tr>
					<td>
						Linux x86 64-bit musl
					</td>
					<td>
						<a href="https://github.com/Matissoss/msat/releases">Link</a>
					</td>
				</tr>
			</table>
			<hr>
			<h2>2. Configure</h2>
			<p> After downloading, choose directory where you want to unzip it and do it.<br>
			Then you will want to head into <i>data/config.toml</i> file and configure it to your needs.
			<br>
			Example <i>data/config.toml</i> configuration:
			</p>
			<code>
				password="test"

				[http_server]
				http_port = 8000
				max_connections = 100
				max_timeout_seconds = 10
				tcp_ip = "127.0.0.1"
				
				[application_server]
				port = 8888
				max_connections = 100 
				max_timeout_seconds = 7
				tcp_ip = "127.0.0.1"
			</code>
			<hr>
			<h2>3. Launch</h2>
			<p>At this point you done everything to setup <i>msat</i>. You can launch application_server and admin_dashboard<br>If you launch admin_dashboard then head into browser and type address: localhost:8000.<br>Log in with password you've set and enjoy.<br>For more info head into docs and admin administrator section</p>

			`
			break;
		default:
			home();
			break;
	}
	window.location.hash = hash;
}

main();
