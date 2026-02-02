import { parseTyped, parseUntyped } from "./typed.js";
import assert from "node:assert";
import { describe, it } from "node:test";
describe("parseUntyped", () => {
    it("parses simple key-value", () => {
        const result = parseUntyped(`host localhost`);
        assert.deepStrictEqual(result, { host: "localhost" });
    });
    it("parses multiple key-values", () => {
        const result = parseUntyped(`
      host localhost
      port 8080
    `);
        assert.deepStrictEqual(result, { host: "localhost", port: "8080" });
    });
    it("parses nested objects", () => {
        const result = parseUntyped(`
      server {
        host localhost
        port 8080
      }
    `);
        assert.deepStrictEqual(result, {
            server: { host: "localhost", port: "8080" },
        });
    });
    it("parses sequences", () => {
        const result = parseUntyped(`ports (80 443 8080)`);
        assert.deepStrictEqual(result, { ports: ["80", "443", "8080"] });
    });
    it("parses unit as null", () => {
        const result = parseUntyped(`nothing @`);
        assert.deepStrictEqual(result, { nothing: null });
    });
});
describe("parseTyped", () => {
    describe("@string", () => {
        it("parses string field", () => {
            const schema = `
        schema {
          @ @object{
            name @string
          }
        }
      `;
            const result = parseTyped(`name "hello world"`, schema);
            assert.strictEqual(result.name, "hello world");
        });
        it("parses bare scalar as string", () => {
            const schema = `
        schema {
          @ @object{
            host @string
          }
        }
      `;
            const result = parseTyped(`host localhost`, schema);
            assert.strictEqual(result.host, "localhost");
        });
    });
    describe("@int", () => {
        it("parses integer field", () => {
            const schema = `
        schema {
          @ @object{
            port @int
          }
        }
      `;
            const result = parseTyped(`port 8080`, schema);
            assert.strictEqual(result.port, 8080);
        });
        it("parses negative integer", () => {
            const schema = `
        schema {
          @ @object{
            offset @int
          }
        }
      `;
            const result = parseTyped(`offset -10`, schema);
            assert.strictEqual(result.offset, -10);
        });
    });
    describe("@float", () => {
        it("parses float field", () => {
            const schema = `
        schema {
          @ @object{
            rate @float
          }
        }
      `;
            const result = parseTyped(`rate 3.14`, schema);
            assert.strictEqual(result.rate, 3.14);
        });
    });
    describe("@bool", () => {
        it("parses true", () => {
            const schema = `
        schema {
          @ @object{
            enabled @bool
          }
        }
      `;
            const result = parseTyped(`enabled true`, schema);
            assert.strictEqual(result.enabled, true);
        });
        it("parses false", () => {
            const schema = `
        schema {
          @ @object{
            enabled @bool
          }
        }
      `;
            const result = parseTyped(`enabled false`, schema);
            assert.strictEqual(result.enabled, false);
        });
    });
    describe("@object", () => {
        it("parses nested object", () => {
            const schema = `
        schema {
          @ @object{
            server @object{
              host @string
              port @int
            }
          }
        }
      `;
            const result = parseTyped(`server { host localhost, port 8080 }`, schema);
            assert.deepStrictEqual(result.server, { host: "localhost", port: 8080 });
        });
        it("parses deeply nested objects", () => {
            const schema = `
        schema {
          @ @object{
            a @object{
              b @object{
                c @string
              }
            }
          }
        }
      `;
            const result = parseTyped(`a { b { c "deep" } }`, schema);
            assert.strictEqual(result.a.b.c, "deep");
        });
    });
    describe("@seq", () => {
        it("parses sequence of strings", () => {
            const schema = `
        schema {
          @ @object{
            tags @seq(@string)
          }
        }
      `;
            const result = parseTyped(`tags (web prod api)`, schema);
            assert.deepStrictEqual(result.tags, ["web", "prod", "api"]);
        });
        it("parses sequence of integers", () => {
            const schema = `
        schema {
          @ @object{
            ports @seq(@int)
          }
        }
      `;
            const result = parseTyped(`ports (80 443 8080)`, schema);
            assert.deepStrictEqual(result.ports, [80, 443, 8080]);
        });
        it("parses sequence of objects", () => {
            const schema = `
        schema {
          @ @object{
            servers @seq(@object{
              host @string
              port @int
            })
          }
        }
      `;
            const result = parseTyped(`servers (
          {host localhost, port 8080}
          {host "example.com", port 443}
        )`, schema);
            assert.deepStrictEqual(result.servers, [
                { host: "localhost", port: 8080 },
                { host: "example.com", port: 443 },
            ]);
        });
    });
    describe("@map", () => {
        it("parses map of strings", () => {
            const schema = `
        schema {
          @ @object{
            env @map(@string)
          }
        }
      `;
            const result = parseTyped(`env { HOME /home/user, PATH /usr/bin }`, schema);
            assert.deepStrictEqual(result.env, {
                HOME: "/home/user",
                PATH: "/usr/bin",
            });
        });
        it("parses map of objects", () => {
            const schema = `
        schema {
          @ @object{
            services @map(@object{
              port @int
              enabled @bool
            })
          }
        }
      `;
            const result = parseTyped(`services {
          web { port 80, enabled true }
          api { port 3000, enabled false }
        }`, schema);
            assert.deepStrictEqual(result.services, {
                web: { port: 80, enabled: true },
                api: { port: 3000, enabled: false },
            });
        });
    });
    describe("@optional", () => {
        it("parses present optional field", () => {
            const schema = `
        schema {
          @ @object{
            name @string
            nickname @optional(@string)
          }
        }
      `;
            const result = parseTyped(`name Alice
         nickname Ali`, schema);
            assert.strictEqual(result.name, "Alice");
            assert.strictEqual(result.nickname, "Ali");
        });
        it("handles missing optional field", () => {
            const schema = `
        schema {
          @ @object{
            name @string
            nickname @optional(@string)
          }
        }
      `;
            const result = parseTyped(`name Alice`, schema);
            assert.strictEqual(result.name, "Alice");
            assert.strictEqual(result.nickname, undefined);
        });
    });
    describe("named types", () => {
        it("resolves named type references", () => {
            const schema = `
        schema {
          @ @object{
            server @Server
          }
          Server @object{
            host @string
            port @int
          }
        }
      `;
            const result = parseTyped(`server { host localhost, port 8080 }`, schema);
            assert.deepStrictEqual(result.server, { host: "localhost", port: 8080 });
        });
        it("resolves deeply nested named types", () => {
            const schema = `
        schema {
          @ @object{
            config @Config
          }
          Config @object{
            server @Server
          }
          Server @object{
            host @string
          }
        }
      `;
            const result = parseTyped(`config { server { host localhost } }`, schema);
            assert.strictEqual(result.config.server.host, "localhost");
        });
        it("uses named types in sequences", () => {
            const schema = `
        schema {
          @ @object{
            servers @seq(@Server)
          }
          Server @object{
            host @string
            port @int
          }
        }
      `;
            const result = parseTyped(`servers (
          {host localhost, port 8080}
          {host "example.com", port 443}
        )`, schema);
            assert.strictEqual(result.servers.length, 2);
            assert.strictEqual(result.servers[0].host, "localhost");
            assert.strictEqual(result.servers[1].port, 443);
        });
        it("uses named types in maps", () => {
            const schema = `
        schema {
          @ @object{
            questions @map(@Question)
          }
          Question @object{
            code @string
            valid @bool
            explanation @string
          }
        }
      `;
            const result = parseTyped(`questions {
          q1 { code "foo bar", valid true, explanation "It works" }
          q2 { code "bad", valid false, explanation "Nope" }
        }`, schema);
            assert.strictEqual(result.questions.q1.valid, true);
            assert.strictEqual(result.questions.q2.valid, false);
            assert.strictEqual(result.questions.q1.explanation, "It works");
        });
    });
    describe("quiz schema", () => {
        it("parses quiz questions format", () => {
            const schema = `
        schema {
          @ @object{
            questions @map(@Question)
          }
          Question @object{
            code @string
            valid @bool
            explanation @string
          }
        }
      `;
            const data = `
        questions {
          three-atoms {
            code "key @tag {}"
            valid false
            explanation "Space between tag and payload"
          }
          simple-kv {
            code "host localhost"
            valid true
            explanation "Simple key-value"
          }
        }
      `;
            const result = parseTyped(data, schema);
            assert.strictEqual(Object.keys(result.questions).length, 2);
            assert.strictEqual(result.questions["three-atoms"].valid, false);
            assert.strictEqual(result.questions["simple-kv"].valid, true);
            assert.strictEqual(result.questions["three-atoms"].code, "key @tag {}");
        });
    });
    describe("heredocs", () => {
        it("parses heredoc content as string", () => {
            const schema = `
        schema {
          @ @object{
            script @string
          }
        }
      `;
            const result = parseTyped(`script <<EOF
echo "hello"
echo "world"
EOF`, schema);
            assert.ok(result.script.includes("hello"));
            assert.ok(result.script.includes("world"));
        });
        it("parses quiz question with heredoc code", () => {
            const schema = `
        schema {
          @ @object{
            questions @map(@Question)
          }
          Question @object{
            code @string
            valid @bool
            explanation @string
          }
        }
      `;
            const data = `
        questions {
          test-q {
            code <<DOC
foo bar
foo baz
DOC
            valid false
            explanation "Duplicate keys"
          }
        }
      `;
            const result = parseTyped(data, schema);
            assert.ok(result.questions["test-q"].code.includes("foo bar"));
            assert.ok(result.questions["test-q"].code.includes("foo baz"));
            assert.strictEqual(result.questions["test-q"].valid, false);
        });
    });
});
