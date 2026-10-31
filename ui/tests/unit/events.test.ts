import {test,expect} from "bun:test";
import {eventNotice} from "../../src/api/events";
test("events require a session and a numeric sequence; they contain no dispatch instruction",()=>{
 expect(eventNotice('{"session":"s","sequence":1,"kind":"reset"}').kind).toBe("reset");
 expect(()=>eventNotice('{"sequence":1}')).toThrow();
 expect(()=>eventNotice('{"session":"s","sequence":"1","kind":"invalidate"}')).toThrow();
});
