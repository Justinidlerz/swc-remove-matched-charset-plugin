import { describe, expect, it } from 'vitest';
import * as swc from '@swc/core';
import * as path from 'node:path';
import pkg from '../package.json';
import * as fs from 'node:fs';
import { Options } from '@swc/core';

const createTransform = async (
  content: string,
  matches: string[],
  replace_with?: string,
  options?: Options,
) => {
  const res = await swc.transform(content, {
    filename: 'input.js',
    sourceMaps: false,
    isModule: true,
    jsc: {
      parser: {
        syntax: 'ecmascript',
        jsx: true,
      },
      transform: {},
      experimental: {
        plugins: [
          [
            require.resolve(path.join(__dirname, '../', pkg.main)),
            {
              matches,
              replace_with,
            },
          ],
        ],
      },
    },
    ...options,
  });

  return res.code;
};

describe('swc-remove-matched-charset-plugin', () => {
  it('Should remove chinese when add chinese charset', async () => {
    const code = await createTransform('const a = "source code中文中文"', [
      '[\u4E00-\u9FFF]',
    ]);

    expect(code).toMatchInlineSnapshot(`
        "var a = "source code";
        "
      `);
  });

  it('Should remove chinese on json properties', async () => {
    const json = {
      value: '中文中文',
      deps1: {
        value: '中文中文1',
      },
    };
    const code = await createTransform(
      `const a = "source code中文中文"; const b = ${JSON.stringify(json)}`,
      ['[\u4E00-\u9FFF]'],
    );

    expect(code).toMatchInlineSnapshot(`
      "var a = "source code";
      var b = {
          "value": "",
          "deps1": {
              "value": "1"
          }
      };
      "
    `);
  });

  it('Should remove url and chinese on json properties', async () => {
    const json = {
      value: '中文中文',
      deps1: {
        value: '中文中文1',
      },
      urls: {
        'baidu.com': 'https://baidu.com/',
        'google.com': 'https://google.com/abc',
      },
    };
    const code = await createTransform(
      `const a = "source code中文中文"; const b = ${JSON.stringify(json)}`,
      ['[\u4E00-\u9FFF]', 'baidu.com|google.com'],
    );

    expect(code).toMatchInlineSnapshot(`
      "function _define_property(obj, key, value) {
          if (key in obj) {
              Object.defineProperty(obj, key, {
                  value: value,
                  enumerable: true,
                  configurable: true,
                  writable: true
              });
          } else {
              obj[key] = value;
          }
          return obj;
      }
      var a = "source code";
      var b = {
          "value": "",
          "deps1": {
              "value": "1"
          },
          "urls": _define_property({
              "": "https:///"
          }, "", "https:///abc")
      };
      "
    `);
  });

  it('Should not remove from import syntax', async () => {
    const code = await createTransform('import * as A from "/中文中文"', [
      '[\u4E00-\u9FFF]',
    ]);

    expect(code).toMatchInlineSnapshot(`
      "import * as A from "/中文中文";
      "
    `);
  });

  it('Should replace with by passed char', async () => {
    const code = await createTransform(
      'console.log("https://www.google.com/url")',
      ['google.com'],
      '*',
    );

    expect(code).toMatchInlineSnapshot(`
      "console.log("https://www.**********/url");
      "
    `);
  });

  it('Should not remove slack', async () => {
    const code = await createTransform('new RegExp("\\\\")', [
      '[\u4E00-\u9FFF]',
    ]);

    expect(code).toMatchInlineSnapshot(`
      "new RegExp("\\\\");
      "
    `);
  });

  it('Should not remove slack from tpl', async () => {
    const a = 'new RegExp(`\\\\中文${b}`)'
    const code = await createTransform(a, [
      '[\u4E00-\u9FFF]',
    ]);

    expect(code).toMatchInlineSnapshot(`
      "new RegExp("\\\\".concat(b));
      "
    `);
  });

  it('Should not transform code when not match', async () => {
    const code = await createTransform(
      fs.readFileSync(path.join(__dirname, './test.js'), 'utf-8'),
      ['[一-鿿]'],
    );

    expect(code).toMatchFileSnapshot(path.join(__dirname, './transformed.js'));
  });

  it('Should not transform code when not match', async () => {
    const code = await createTransform(
      fs.readFileSync(path.join(__dirname, './test.js'), 'utf-8'),
      ['[一-鿿]'],
      '',
      {
        env: {
          targets: [
            'chrome >= 87',
            'edge >= 88',
            'firefox >= 78',
            'safari >= 14',
          ],
          mode: undefined,
        },
      },
    );

    expect(code).toMatchFileSnapshot(path.join(__dirname, './transformed-es.js'));
  });

  it('Should remove chinese in jsx', async() => {
    const app = `
      const app = () => {
        return (
          <div>
            中文中文
            <p>english</p>
          </div>
        )
      } 
    `
    const code = await createTransform(
        app,
        ['[一-鿿]'],
        '',
        {
          env: {
            targets: [
              'chrome >= 87',
              'edge >= 88',
              'firefox >= 78',
              'safari >= 14',
            ],
            mode: undefined,
          },
        },
    );

    expect(code).toMatchInlineSnapshot(`
      "const app = ()=>{
          return /*#__PURE__*/ React.createElement("div", null, /*#__PURE__*/ React.createElement("p", null, "english"));
      };
      "
    `)
  })
});
