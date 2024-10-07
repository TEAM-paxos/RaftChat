import { MsgHandler } from './message.js';
import * as utils from './utils.js'

export class Engine{
    id;
    userId;
    committedIndex;
    
    serverNameList = [];

    socket;
    msgHandler;

    // Icons made by Freepik from www.flaticon.com
    BOT_IMG = "https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Headshot__0087_cropped_leslie-lamport.jpg";
    PERSON_IMG = "https://www.gravatar.com/avatar/056effcac7fca237926f57ba2450429a";

    // html attributes
    constructor(serverNameList){
        this.id = "unique_id";
        this.userId = "default userId";
        this.committedIndex = 0;
        this.serverNameList = serverNameList;  

        this.msgerForm = utils.get(".msger-inputarea");
        this.msgerInput = utils.get(".msger-input");
        this.msgerChat = utils.get(".msger-chat");
        this.portForm = utils.get(".port-inputarea");
        this.portInput = utils.get(".port-input"); 

        this.msgHandler = new MsgHandler();

        this.portForm.addEventListener("submit", event => {
            event.preventDefault();
      
            const port = this.portInput.value;
            if(isNaN(port)) return;
      
            const res = this.connectWS(port);
        })

        this.msgerForm.addEventListener("submit", this.sendToServer.bind(this));
    }

    connectWS(port){
        this.socket = new WebSocket("ws://localhost:"+port);
        console.log("connect to " + "localhost:"+port);

        this.socket.onopen = () => {
            console.log("WebSocket is open");
            this.portInput.style.backgroundColor =  'rgb(0, 196, 65)';
        };

        this.socket.onmessage = (event) => {
            console.log("Message from server:", event.data);
            this.botResponse(event.data);
        };

        this.socket.onerror = (error) => {
            // repeat connect the other server
            console.log("Error has occured:", error);
            this.portInput.style.backgroundColor = 'red';
        };

        this.socket.onclose = (error) => {
            console.log("Connection is closed", error);
            this.portInput.style.backgroundColor = 'red';
        }
    }

    sendToServer(event){
        event.preventDefault();
        console.log(this);
        const msgText = this.msgerInput.value;
        if (!msgText) return;
        
        
        // 0. update html
        this.appendMessage(this.user, this.PERSON_IMG, "right", this.msgText);
        this.msgerInput.value = "";

        // 1. Save message into msgHandler.
        this.msgHandler.append(
            this.id, this.userId, msgText, this.committedIndex
        )

        // 2. Send all messages in msgHandler.
        let msgArray = this.msgHandler.toJsonArray();
        let json = {
            committed_index : this.committedIndex,
            messages : msgArray,
        }

        this.socket.send(JSON.stringify(json));
        console.log(JSON.stringify(json))
    }

    appendMessage(name, img, side, text) {
        //   Simple solution for small apps
        const msgHTML = `
          <div class="msg ${side}-msg">
            <div class="msg-img" style="background-image: url(${img})"></div>
      
            <div class="msg-bubble">
              <div class="msg-info">
                <div class="msg-info-name">${name}</div>
                <div class="msg-info-time">${utils.formatDate(new Date())}</div>
              </div>
      
              <div class="msg-text">${text}</div>
            </div>
          </div>
        `;
      
        this.msgerChat.insertAdjacentHTML("beforeend", msgHTML);
        this.msgerChat.scrollTop += 500;
    }

    botResponse(msgText) {
        // const r = random(0, BOT_MSGS.length - 1);
        // const msgText = BOT_MSGS[r];
        const delay = msgText.split(" ").length * 100;
      
        setTimeout(() => {
          this.appendMessage(this.BOT_NAME, this.BOT_IMG, "left", msgText);
        }, delay);
      }
}