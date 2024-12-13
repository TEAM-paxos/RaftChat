import * as ms from '../static/message.js';
import * as storage from '../static/storage.js';

class LocalStorageMock {
  constructor() {
    this.store = {};
  }

  clear() {
    this.store = {};
  }

  getItem(key) {
    return this.store[key] || null;
  }

  setItem(key, value) {
    this.store[key] = String(value);
  }

  removeItem(key) {
    delete this.store[key];
  }
}

Object.defineProperty(global, 'localStorage', {
  value: new LocalStorageMock(),
});

let msgHandler;
let _storage;

describe("message test", () => {
  beforeEach(() => {
    _storage = new storage.Storage();
    _storage.reset();
    msgHandler = new ms.MsgHandler(_storage);
  });
  
  afterEach(() => {
    msgHandler = "";
  });
  
  test('append test', () => {
    
    msgHandler.append("1", "high", "hello~");
    msgHandler.append("2", "high", "hello~");

    let msg1 = new ms.Msg("1", "high", "hello~", 1);
    let msg2 = new ms.Msg("2", "high", "hello~", 2);

    expect(msgHandler.getQue[0].isEqual(msg1)).toEqual(true);
    expect(msgHandler.getQue[1].isEqual(msg2)).toEqual(true);

  });

  test('cleanUp test', () => {
    let temp = [];
    for(let i = 0; i<10;i++){
      msgHandler.append("1", "high", "hello~");
      let msg = new ms.Msg("1", "high", "hello~", i+1);
      temp.push(msg);
    }

    for(let i=0;i<msgHandler.queSize();i++){
      expect(msgHandler.getQue[i].isEqual(temp[i])).toEqual(true);
    }

    for(let i=0;i<10;i++){
      msgHandler.cleanUp(i+1);
      expect(msgHandler.queSize()).toEqual(9-i);

      temp.shift();

      for(let i=0;i<msgHandler.queSize();i++){
        expect(msgHandler.getQue[i].isEqual(temp[i])).toEqual(true);
      }
    }

  })

  test('json test', ()=> {
    let temp = [];
    for(let i = 0; i<3;i++){
      msgHandler.append("1", "high", "hello~");
      let msg = new ms.Msg("1", "high", "hello~", i+1);
      temp.push(msg);
    }

    expect(temp[0].toJson()).toEqual(
      {
        id: '1',
        user_id: 'high',
        content: 'hello~',
        time: temp[0].time,
        time_stamp: 1
      }
    )

    expect(msgHandler.toJsonArray()).toEqual(
      [
        temp[0].toJson(),
      ]
    )
    expect(msgHandler.toJsonArray()).toEqual(
      [
        temp[1].toJson(),
      ]
    )
    expect(msgHandler.toJsonArray()).toEqual(
      [
        temp[2].toJson(),
      ]
    )

  })
})