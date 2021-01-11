let https = require("https");

let baseurl = "https://escapefromtarkov.gamepedia.com/api.php?action=query&format=json&list=allpages";

let apcontinue = "\"Big Stick\" 9x19 magazine for Glock 9x19";
//let apcontinue = "\"Big Stick\" 9x19 magazine for Glock 9x19";

function GET(url) {
    return new Promise((resolve, reject) => {
        let d = https.get(url);

        let data = "";

        d.on("data", c => { data += chunk; });
        d.on("end", () => { resolve(JSON.parse(data)); });
        d.on("error", e => { reject(e); });
    });
}

async function main() {
    console.log("ya");
    for(;;){
        let url = baseurl + "&apcontinue=" + encodeURIComponent(apcontinue)

        let d = await GET(url);

        console.log("data get");
        console.log(d);

        break;
    }
}

main();
