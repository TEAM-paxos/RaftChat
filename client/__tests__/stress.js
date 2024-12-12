import fetch from 'node-fetch'
import { Engine } from '../static/engine.js'
import { Storage } from '../static/storage.js';

let engine = new Engine(new Storage(), false);

fetch("http://127.0.0.1:3000/get_info")
    .then((response) => response.json())
    .then((data) => {
    console.log(data);
    
    engine.setup(data, () => {
        console.log("-----------------------------");
        engine.sendToServer("hi");
    });
});

