pub const INSTALL: &str = r#"(() => {
  let state=0x9e3779b9;
  const next=()=>{state^=state<<13;state^=state>>>17;state^=state<<5;return state>>>0};
  Math.random=()=>next()/4294967296;
  const fill=array=>{
    const bytes=new Uint8Array(array.buffer,array.byteOffset,array.byteLength);
    for(let index=0;index<bytes.length;index++)bytes[index]=next()&255;
    return array;
  };
  Object.defineProperty(Crypto.prototype,'getRandomValues',{value:fill});
  Object.defineProperty(Crypto.prototype,'randomUUID',{value:()=>{
    const bytes=fill(new Uint8Array(16));
    bytes[6]=(bytes[6]&15)|64;bytes[8]=(bytes[8]&63)|128;
    const value=[...bytes].map(byte=>byte.toString(16).padStart(2,'0')).join('');
    return `${value.slice(0,8)}-${value.slice(8,12)}-${value.slice(12,16)}-`+
      `${value.slice(16,20)}-${value.slice(20)}`;
  }});
  const NativeDate=Date;
  const epoch=1700000000000;
  globalThis.Date=class extends NativeDate {
    constructor(...args){super(...(args.length?args:[epoch]))}
    static now(){return epoch}
  };
})()"#;
