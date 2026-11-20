// Splash shown while the live app loads; becomes the offline page when it can't be reached.
const APP = "https://dayditto.prototypolab.com";
const RETRY_MS = 5000;

const offline = document.getElementById("offline");
let timer;

async function connect() {
  clearTimeout(timer);
  try {
    // An opaque response is enough: it means the server is reachable.
    await fetch(`${APP}/login`, { mode: "no-cors", cache: "no-store" });
    location.replace(`${APP}/`);
  } catch {
    offline.hidden = false;
    document.body.classList.add("offline");
    timer = setTimeout(connect, RETRY_MS);
  }
}

document.getElementById("retry").addEventListener("click", connect);
connect();
