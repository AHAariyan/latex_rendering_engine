# Browser test

Loads `demo.html` with the packed `mathcore` package in Chromium, Firefox and
WebKit (Safari's engine) and checks that every `<math-tex>` draws, that a
display formula breaks to its container on the first frame, that speech
follows the page's `lang`, that errors show, and that nothing logs an error.

    cargo xtask sdk web
    cp platforms/web/browser-test/demo.html dist/web/package/
    cd platforms/web/browser-test && npm install playwright@1 && npx playwright install chromium firefox webkit
    node check.mjs ../../../dist/web/package
