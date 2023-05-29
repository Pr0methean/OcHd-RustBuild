// svgo.config.js
module.exports = {
    multipass: true, // boolean. false by default
    js2svg: {
        indent: 0, // string with spaces or number of spaces. 4 by default
    },
    plugins: [
        // set of built-in plugins enabled by default
        'preset-default',
        'removeOffCanvasPaths',
        'removeUnusedNS',
        {
            name: 'cleanupNumericValues',
            params: {
                overrides: {
                    floatPrecision: 1,
                    convertToPx: true,
                    defaultPx: true
                }
            }
        }
    ],
};