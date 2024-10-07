// This 
export class MsgHandler{
    #msgQue = []
    #timeStamp
    constructor(){
        this.#timeStamp = 0;
    }

    get getQue(){
        return [...this.#msgQue];
    }


    append(id, user_id, content){
        this.#timeStamp += 1;
        this.#msgQue.push(new Msg(id, user_id, content, this.#timeStamp));
    }

   
    toJsonArray(){
        let temp = [];
        for(let i=0;i<this.#msgQue.length;i++){
            temp.push(this.#msgQue[i].toJson());
        }
        return temp;
    }

    queSize(){
        return this.#msgQue.length;
    }

    cleanUp(timeStamp){
        for (let i = this.#msgQue.length - 1; i >= 0; i--) {
            if (this.#msgQue[i].timeStamp === timeStamp) {
                this.#msgQue.splice(i, 1); // 해당 인덱스의 요소 삭제
            }
        }
    }
}

export class Msg {
    #id;
    #userId;
    #content;
    #time;
    #timeStamp;

    constructor(id, userId, content, timeStamp){
        this.#id = id;
        this.#userId = userId;
        this.#content = content;
        this.#time = Math.floor(Date.now() / 1000);
        this.#timeStamp = timeStamp;
    }

    // Getter
    get id() {
        return this.#id;
    }

    get userId() {
        return this.#userId;
    }

    get content() {
        return this.#content;
    }

    get time() {
        return this.#time;
    }

    get timeStamp() {
        return this.#timeStamp;
    }

    toJson(){
        return {
            id: this.#id,
            user_id: this.#userId,
            content: this.#content,
            time: this.#time,
            time_stamp: this.#timeStamp,
        };
    }

    isEqual(other) {
        if (!(other instanceof Msg)) return false; // 다른 타입일 경우 false
        return (
            this.id === other.id &&
            this.user_id === other.user_id &&
            this.content === other.content &&
            this.time === other.time &&
            this.timeStamp === other.timeStamp
        );
    }
}
