import type { Plugin } from "vite";
import postcssrc from "postcss-load-config";
import pxToRem from "postcss-pxtorem";

function isObject(value: unknown): value is Record<string, any> {
  return Object.prototype.toString.call(value) === "[object Object]";
}

type Options = {
  rootValue: number;
  unitPrecision: number;
  propList: string[];
  selectorBlackList: string[];
  replace: boolean;
  mediaQuery: boolean;
  minPixelValue: number;
  exclude: (path: string) => boolean;
};

interface IPluginOptions {
  options: Partial<Options>;
}

export default function pxToRemOrVwPlugin(opts: IPluginOptions): Plugin {
  return {
    name: "vite-plugin-css-px2rem",
    config() {
      return {
        css: {},
      };
    },
    async configResolved(config) {
      // must be exist because defined in config hook
      const cssOptions = config.css!;
      const postCssOptions = cssOptions.postcss;

      //   https://www.npmjs.com/package/postcss-pxtorem
      const pluginInfo = pxToRem(opts.options);

      if (isObject(postCssOptions)) {
        postCssOptions.plugins = postCssOptions.plugins || [];
        postCssOptions.plugins.push(pluginInfo);
      } else {
        const searchPath =
          typeof postCssOptions === "string" ? postCssOptions : config.root;
        //load postcss config
        try {
          const result = await postcssrc({}, searchPath);
          result.plugins.push(pluginInfo);
          cssOptions.postcss = result as any;
        } catch (error) {
          //no found postcss config
          cssOptions.postcss = {
            plugins: [pluginInfo],
          };
        }
      }
    },
    // transform(code, id) {
    //   try {
    //     test();
    //     if (StyleFileSuffixRegex.test(id)) {
    //       const tree = cssTree.parse(code) as cssTree.StyleSheet;

    //       cssTree.walk(tree, {
    //         enter(node: any) {
    //           if (node.unit === "px") {
    //             node.value = `${Number(node.value) / rootValue}`;
    //             node.unit = "rem";
    //           }
    //         },
    //       });

    //       return {
    //         code: cssTree.generate(tree),
    //       };
    //     }
    //   } catch (ex) {
    //     console.log(ex);
    //   }

    //   return {
    //     code,
    //   };
    // },
    transformIndexHtml() {
      return;
    },
  };
}
