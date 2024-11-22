/*!
 * Copyright (c) 2024 "Bumsung Baek <highcloud100@inha.edu>,
                         SoonWon Moon < damhiya@gmail.com> "
 * Licensed under the MIT License.
 * See the LICENSE file in the project root for more information.
*/

import { MsgHandler } from './message.js';
import * as utils from './utils.js'
import { Storage } from './storage.js';

export class Engine {
    id;
    userId;
    committedIndex;
    current_host;
    current_port;
    serverNameList = [];

    socket;
    msgHandler;

    // Icons made by Freepik from www.flaticon.com
    OTHER_IMG = "https://www.microsoft.com/en-us/research/wp-content/uploads/2016/02/Headshot__0087_cropped_leslie-lamport.jpg";
    USER_IMG = "https://www.gravatar.com/avatar/056effcac7fca237926f57ba2450429a";

    // html attributes
    constructor(serverNameList) {
        this.id = utils.getRandomString(7);
        this.userId = utils.getRandomString(7);
        this.committedIndex = 0; // committed index that client want to receive
        this.serverNameList = serverNameList;

        this.msgerForm = utils.get(".msger-inputarea");
        this.msgerInput = utils.get(".msger-input");
        this.msgerChat = utils.get(".msger-chat");
        this.portForm = utils.get(".port-inputarea");
        this.portInput = utils.get(".port-send-btn");
        this.notCommittedChat = utils.get(".nc-msger-chat");
        this.msgHandler = new MsgHandler();
        this.storage = new Storage();


        this.portForm.addEventListener("submit", event => {
            event.preventDefault();

            // const port = this.portInput.value;
            // if(isNaN(port)) return;

            console.log("call func");
            fetch("/get_info")
                .then((response) => response.json())
                .then((data) => {
                    console.log(data);
                    this.info = data;

                    // load from storage
                    if (this.info.version == this.storage.getServerVersion()) {
                        try {
                            this.committedIndex = parseInt(this.storage.getLatestIdx());
                            this.id = this.storage.getId();
                            this.userId = this.storage.getUid();

                            console.log(this.committedIndex + " " + this.id + " " + this.userId);

                            for (let i = 0; i < this.committedIndex; i++) {
                                let msg = this.storage.getMessage(i);

                                if (msg.id == this.id) {
                                    this.appendCommittedMessage(msg.user_id, this.USER_IMG, "right", msg.content, msg.time, i);
                                }
                                else {
                                    this.appendCommittedMessage(msg.user_id, this.OTHER_IMG, "left", msg.content, msg.time, i);
                                }
                            }
                        }
                        catch (err) {
                            console.log("reading storage" + err);
                        }

                    } else {
                        this.storage.clear();
                        this.storage.setIdUid(this.id, this.userId);
                        this.storage.setServerVersion(this.info.version);
                        this.storage.setLatestIdx(0);
                        this.storage.setTimeStamp(0);
                    }


                    this.current_port = this.info.socket_ports[this.info.self_domain_idx];
                    this.current_host = this.info.domains[this.info.self_domain_idx];

                    this.connectWS(window.location.hostname, this.current_port);
                })
        })


        this.msgerForm.addEventListener("submit", this.sendToServer.bind(this));
    }

    connectWS(host, port) {
        this.socket = new WebSocket("ws://" + host + ":" + port);
        console.log("connect to " + host + " : " + port);
        this.portInput.disabled = true;

        this.socket.onopen = () => {
            console.log("WebSocket is open");
            this.portInput.style.backgroundColor = 'rgb(118, 255, 167)';
            this.interval = setInterval(this.retransmission.bind(this), 5000);
        };

        this.socket.onmessage = (event) => {
            console.log("Message from server:", event.data);
            this.updateState(event.data);
        };

        this.socket.onerror = (error) => {
            // repeat connect the other server
            console.log("Error has occured:", error);
            this.portInput.style.backgroundColor = 'red';
            this.retry_connection();
        };

        this.socket.onclose = (error) => {
            console.log("Connection is closed", error);
            this.portInput.style.backgroundColor = 'red';
            this.retry_connection();
        }
    }

    retry_connection() {
        let new_host;
        let new_port;
        while (1) {
            let rand_idx = Math.floor((Math.random() * this.info.domains.length));
            new_host = this.info.domains[rand_idx];
            new_port = this.info.socket_ports[rand_idx];

            console.log(new_host + " " + new_port + ":" + this.current_host);

            if (new_host != this.current_host || new_port != this.current_port) {
                break;
            }
        }
        // alert("repeat connect the" + new_host + " server")
        this.connectWS(new_host, new_port);
        this.current_host = new_host;
        this.current_port = new_port;
    }

    retransmission() {
        this.msgHandler.timeoutCheck();

        let msgArray = this.msgHandler.toJsonArray();
        let json = {
            committed_index: this.committedIndex,
            messages: msgArray,
        }

        // length is 0 then return
        if (msgArray.length == 0) {
            return;
        }

        this.socket.send(JSON.stringify(json));
        console.log("retrans: " + JSON.stringify(json))
    }

    sendToServer(event) {
        event.preventDefault();

        const msgText = this.msgerInput.value;
        if (!msgText) return;

        // 0. Save message into msgHandler.
        let timestamp = this.msgHandler.append(
            this.id, this.userId, msgText
        )

        // 1. Update html
        this.appendNotCommittedMessage(this.userId, this.USER_IMG, "right", msgText, timestamp);
        this.msgerInput.value = "";

        // 2. Send messages from msgHandler.
        let msgArray = this.msgHandler.toJsonArray();

        let json = {
            committed_index: this.committedIndex,
            messages: msgArray,
        }

        this.socket.send(JSON.stringify(json));
        console.log("send to server:" + JSON.stringify(json))
    }

    updateState(serverMsgs) {
        let serverMsg = JSON.parse(serverMsgs);
        let deletedFlag = false;

        // 1. Update committed index
        // this.committedIndex = serverMsg.committed_index+1;

        // 2. Update html
        for (let i = 0; i < serverMsg.length; i++) {
            let msg = serverMsg[i].message;
            let msgs_committed_idx = serverMsg[i].committed_index;

            if (msgs_committed_idx != this.committedIndex) {
                continue;
            }
            this.committedIndex++;

            // save in storage
            this.storage.saveMessage(msgs_committed_idx, msg);
            this.storage.setLatestIdx(this.committedIndex);


            if (msg.id == this.id) {
                // Clean up msgHandler
                this.msgHandler.cleanUp(msg.time_stamp);

                try {
                    // Clean up not committed chat
                    let parent = document.getElementById("nc-chat-area");
                    let child = document.getElementById(msg.time_stamp);
                    parent.removeChild(child);
                }
                catch (err) {
                    console.log("there is no element to delete");
                    console.log(err);
                }

                this.appendCommittedMessage(msg.user_id, this.USER_IMG, "right", msg.content, msg.time, msgs_committed_idx);
                deletedFlag = true;
            }
            else {
                this.appendCommittedMessage(msg.user_id, this.OTHER_IMG, "left", msg.content, msg.time, msgs_committed_idx);
            }
        }

        // 3. double msg size  
        if (deletedFlag) {
            this.msgHandler.doubleMsgSize();
        }

        // // 4. ack committed idx 
        // let json = {
        //     committed_index: this.committedIndex,
        //     messages: [],
        // }

        // this.socket.send(JSON.stringify(json));
        // console.log("send to server:" + JSON.stringify(json))
    }

    appendNotCommittedMessage(name, img, side, text, time_stamp) {
        const msgHTML = `
        <div class="msg ${side}-msg" id=${time_stamp}>
          <div class="msg-img" style="background-image: url(${img})"></div>
    
          <div class="nc-msg-bubble">
            <div class="msg-info">
              <div class="msg-info-name">${name}</div>
              <div class="msg-info-time">${utils.formatDate(new Date())}</div>
            </div>
    
            <div class="msg-text"><pre>${utils.insertLineBreaks(text)}</pre> </div>
          </div>
        </div>
      `;

        this.notCommittedChat.insertAdjacentHTML("beforeend", msgHTML);
        this.notCommittedChat.scrollTop += 500;
    }

    appendCommittedMessage(name, img, side, text, time, committed_index) {
        //   Simple solution for small apps
        const msgHTML = `
          <div class="msg ${side}-msg">
            <div class="msg-img" style="background-image: url(${img})"></div>
      
            <div class="msg-bubble">
              <div class="msg-info">
                <div class="msg-info-name">${name}</div>
                <div class="msg-info-time">${committed_index} / ${utils.formatDate(new Date(time))}</div>
              </div>
      
              <div class="msg-text">${utils.insertLineBreaks(text)}</div>
            </div>
          </div>
        `;

        this.msgerChat.insertAdjacentHTML("beforeend", msgHTML);
        this.msgerChat.scrollTop += 500;
    }
}