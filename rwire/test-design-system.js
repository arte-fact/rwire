const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();

  try {
    await page.goto('http://127.0.0.1:9000');

    // Wait for WebSocket to inject styles
    await page.waitForTimeout(1000);

    // Get computed styles for the root element
    const rootStyles = await page.evaluate(() => {
      const root = document.documentElement;
      const styles = getComputedStyle(root);

      // Check specific CSS variables
      const vars = [
        '--rw-bg-app',
        '--rw-text-default',
        '--rw-neutral-1',
        '--rw-blue-9',
        '--rw-space-4',
        '--rw-radius-md',
        '--rw-accent-9',
        '--rw-text-high',
        '--rw-border-default',
        '--rw-leading-normal'
      ];

      const results = {};
      vars.forEach(v => {
        results[v] = styles.getPropertyValue(v).trim();
      });

      return results;
    });

    console.log('CSS Variables Check:');
    console.log('===================');
    for (const [varName, value] of Object.entries(rootStyles)) {
      if (value === '') {
        console.log(`❌ ${varName}: EMPTY`);
      } else {
        console.log(`✓ ${varName}: ${value}`);
      }
    }

    // Check if buttons are styled
    const buttonStyles = await page.evaluate(() => {
      const button = document.querySelector('button');
      if (!button) return null;

      const styles = getComputedStyle(button);
      return {
        backgroundColor: styles.backgroundColor,
        color: styles.color,
        padding: styles.padding,
        borderRadius: styles.borderRadius
      };
    });

    if (buttonStyles) {
      console.log('\nButton Styles:');
      console.log('=============');
      console.log(JSON.stringify(buttonStyles, null, 2));
    }

    // Take a screenshot
    await page.screenshot({ path: '/tmp/design-system.png', fullPage: true });
    console.log('\nScreenshot saved to /tmp/design-system.png');

  } catch (error) {
    console.error('Error:', error.message);
  } finally {
    await browser.close();
  }
})();
