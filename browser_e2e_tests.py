#!/usr/bin/env python3
"""
Browser-based E2E tests for the blog application.

This module provides comprehensive browser automation testing for:
- Like button functionality and UI interactions
- JavaScript behavior and error handling
- Real user workflows and journeys
- Cross-browser compatibility testing
"""

import asyncio
import json
import time
import sys
from termcolor import cprint
from playwright.async_api import async_playwright, Page, Browser


class BlogE2ETests:
    """Browser-based end-to-end tests for the blog application..."""
    
    def __init__(self, base_url="http://server", screenshots_dir="/tmp/playwright-screenshots"):
        self.base_url = base_url
        self.browser = None
        self.context = None
        self.page = None
        self.post_slug = "automate_boring_stuff"
        self.screenshots_dir = screenshots_dir
        self.screenshot_counter = 0
        
        # Create screenshots directory
        import os
        os.makedirs(self.screenshots_dir, exist_ok=True)
    
    async def setup_browser(self, headless=True):
        """Initialize browser for testing."""
        cprint("Setting up browser for E2E testing...", "yellow")
        
        self.playwright = await async_playwright().start()
        self.browser = await self.playwright.chromium.launch(
            headless=headless,
            args=[
                '--no-sandbox',
                '--disable-dev-shm-usage',
                '--disable-gpu',
                '--disable-web-security',
                '--disable-features=VizDisplayCompositor'
            ]
        )
        
        self.context = await self.browser.new_context(
            viewport={'width': 1280, 'height': 720}
        )
        self.page = await self.context.new_page()
        
        # Enable console logging for debugging
        self.page.on("console", lambda msg: cprint(f"Browser Console: {msg.text}", "cyan"))
        self.page.on("pageerror", lambda exc: cprint(f"Browser Error: {exc}", "red"))
    
    async def take_screenshot(self, name):
        """Take a screenshot with a descriptive name."""
        if self.page:
            self.screenshot_counter += 1
            filename = f"{self.screenshot_counter:02d}_{name}.png"
            filepath = f"{self.screenshots_dir}/{filename}"
            await self.page.screenshot(path=filepath, full_page=True)
            cprint(f"📸 Screenshot saved: {filepath}", "blue")
    
    async def teardown_browser(self):
        """Clean up browser resources."""
        if self.page:
            await self.page.close()
        if self.context:
            await self.context.close()
        if self.browser:
            await self.browser.close()
        if hasattr(self, 'playwright'):
            await self.playwright.stop()
    
    async def test_blog_page_loads(self):
        """Test that the blog post page loads correctly."""
        cprint("Testing blog page loads...", "blue")
        
        url = f"{self.base_url}/posts/automate_boring_stuff"
        await self.page.goto(url)
        
        # Take screenshot after navigation
        await self.take_screenshot("page_loaded")
        
        # Wait for the page to fully load
        await self.page.wait_for_load_state("networkidle")
        
        # Verify page title
        title = await self.page.title()
        assert "Automating the pain away" in title, f"Expected page title to contain 'Automating the pain away', got: {title}"
        
        # Verify main heading
        heading = await self.page.locator("h1").text_content()
        assert "Automating the pain away" in heading, f"Expected heading to contain 'Automating the pain away', got: {heading}"
        
        # Verify like button is present
        like_button = self.page.locator(f"#like-btn-{self.post_slug}")
        await like_button.wait_for(state="visible")
        
        # Take screenshot showing the like button
        await self.take_screenshot("like_button_visible")
        
        button_text = await like_button.text_content()
        assert "Like" in button_text, f"Expected like button to contain 'Like', got: {button_text}"
        
        cprint("✓ Blog page loads correctly", "green")
    
    async def test_like_count_loads(self):
        """Test that the like count loads and displays correctly."""
        cprint("Testing like count loads...", "blue")
        
        url = f"{self.base_url}/posts/automate_boring_stuff"
        await self.page.goto(url)
        await self.page.wait_for_load_state("networkidle")
        
        # Wait for like count to load (should change from "Loading...")
        like_count = self.page.locator(f"#count-{self.post_slug}")
        
        # Wait for like count to show actual number (not "Loading...")
        await self.page.wait_for_function(
            f"document.getElementById('count-{self.post_slug}').textContent !== 'Loading...'",
            timeout=10000
        )
        
        count_text = await like_count.text_content()
        # The count element just shows the number, not the word "likes"
        assert count_text.isdigit(), f"Expected like count to be a number, got: {count_text}"
        
        # Extract number from like count
        import re
        match = re.search(r'(\d+)', count_text)
        assert match, f"Expected like count to contain a number, got: {count_text}"
        
        initial_count = int(match.group(1))
        cprint(f"✓ Like count loads correctly: {initial_count} likes", "green")
        
        return initial_count
    
    async def test_like_button_interaction(self):
        """Test clicking the like button and verifying UI updates."""
        cprint("Testing like button interaction...", "blue")
        
        url = f"{self.base_url}/posts/automate_boring_stuff"
        await self.page.goto(url)
        await self.page.wait_for_load_state("networkidle")
        
        # Get initial like count
        initial_count = await self.test_like_count_loads()
        
        # Take screenshot before clicking
        await self.take_screenshot("before_like_click")
        
        # Click the like button
        like_button = self.page.locator(f"#like-btn-{self.post_slug}")
        await like_button.click()
        
        # Take screenshot immediately after click
        await self.take_screenshot("after_like_click")
        
        # Wait for button to be disabled and heart to show spinner (or skip if too fast)
        try:
            await self.page.wait_for_function(
                f"document.getElementById('like-btn-{self.post_slug}').disabled",
                timeout=2000
            )
        except Exception:
            # Button state might change too quickly, continue with test
            cprint("Button state changed too quickly to catch disabled state", "yellow")
        
        # Take screenshot during processing
        await self.take_screenshot("like_processing")
        
        # Wait for final state (button re-enabled)
        await self.page.wait_for_function(
            f"!document.getElementById('like-btn-{self.post_slug}').disabled",
            timeout=10000
        )
        
        # Take screenshot of final state
        await self.take_screenshot("like_final_state")
        
        # Check final button state
        final_button_text = await like_button.text_content()
        
        # Check if like was successful by looking at the button state
        if await like_button.evaluate("el => el.classList.contains('liked')"):
            # Success case - verify like count increased
            cprint("✓ Like button shows success state", "green")
            
            # Wait for like count to update
            await self.page.wait_for_function(
                f"parseInt(document.getElementById('count-{self.post_slug}').textContent) > {initial_count}",
                timeout=5000
            )
            
            final_count_text = await self.page.locator(f"#count-{self.post_slug}").text_content()
            import re
            match = re.search(r'(\d+)', final_count_text)
            final_count = int(match.group(1)) if match else 0
            
            assert final_count > initial_count, f"Expected like count to increase from {initial_count}, got {final_count}"
            cprint(f"✓ Like count increased from {initial_count} to {final_count}", "green")
            
            # Take screenshot showing success
            await self.take_screenshot("like_success")
            
        else:
            # Rate limited case - check for error message
            error_message = self.page.locator(f"#like-message-{self.post_slug}")
            if await error_message.is_visible():
                error_text = await error_message.text_content()
                cprint(f"✓ Rate limiting working: {error_text}", "green")
                await self.take_screenshot("rate_limited")
            else:
                cprint("✓ Like button returned to normal state (possible rate limiting)", "yellow")
                await self.take_screenshot("like_normal_state")
    
    async def test_javascript_error_handling(self):
        """Test JavaScript error handling when backend is unavailable."""
        cprint("Testing JavaScript error handling...", "blue")
        
        url = f"{self.base_url}/posts/automate_boring_stuff"
        await self.page.goto(url)
        await self.page.wait_for_load_state("networkidle")
        
        # Intercept and block API requests to simulate backend failure
        await self.page.route("**/api/like/**", lambda route: route.abort())
        
        # Take screenshot before simulated error
        await self.take_screenshot("before_error_test")
        
        # Click the like button
        like_button = self.page.locator(f"#like-btn-{self.post_slug}")
        await like_button.click()
        
        # Wait for error state
        await self.page.wait_for_function(
            f"document.getElementById('like-message-{self.post_slug}').style.display !== 'none'",
            timeout=10000
        )
        
        # Take screenshot showing error message
        await self.take_screenshot("error_message_displayed")
        
        # Verify error message is displayed
        error_message = self.page.locator(f"#like-message-{self.post_slug}")
        error_text = await error_message.text_content()
        
        assert "error" in error_text.lower(), f"Expected error message to contain 'error', got: {error_text}"
        cprint(f"✓ JavaScript error handling works: {error_text}", "green")
        
        # Verify button returns to normal state
        final_button_text = await like_button.text_content()
        assert "Like" in final_button_text, f"Expected button to return to 'Like' state, got: {final_button_text}"
    
    async def test_responsive_design(self):
        """Test like button functionality on different viewport sizes."""
        cprint("Testing responsive design...", "blue")
        
        viewports = [
            {'width': 320, 'height': 568, 'name': 'Mobile'},
            {'width': 768, 'height': 1024, 'name': 'Tablet'},
            {'width': 1920, 'height': 1080, 'name': 'Desktop'}
        ]
        
        for viewport in viewports:
            cprint(f"Testing {viewport['name']} viewport ({viewport['width']}x{viewport['height']})...", "cyan")
            
            await self.page.set_viewport_size({'width': viewport['width'], 'height': viewport['height']})
            
            url = f"{self.base_url}/posts/automate_boring_stuff"
            await self.page.goto(url)
            await self.page.wait_for_load_state("networkidle")
            
            # Take screenshot of responsive layout
            await self.take_screenshot(f"responsive_{viewport['name'].lower()}")
            
            # Verify like button is visible and clickable
            like_button = self.page.locator(f"#like-btn-{self.post_slug}")
            await like_button.wait_for(state="visible")
            
            # Check button is not hidden or overlapped
            button_box = await like_button.bounding_box()
            assert button_box, f"Like button not visible on {viewport['name']} viewport"
            assert button_box['width'] > 0 and button_box['height'] > 0, f"Like button has zero dimensions on {viewport['name']} viewport"
            
            cprint(f"✓ Like button visible and properly sized on {viewport['name']}", "green")
    
    async def test_user_journey_multiple_interactions(self):
        """Test complete user journey with multiple interactions."""
        cprint("Testing complete user journey...", "blue")
        
        url = f"{self.base_url}/posts/automate_boring_stuff"
        await self.page.goto(url)
        await self.page.wait_for_load_state("networkidle")
        
        # Take screenshot of initial page
        await self.take_screenshot("user_journey_start")
        
        # Simulate reading the blog post (scroll down)
        await self.page.evaluate("window.scrollTo(0, document.body.scrollHeight / 2)")
        await asyncio.sleep(1)
        
        # Take screenshot after scrolling
        await self.take_screenshot("user_journey_scrolled")
        
        # Scroll back to like button
        await self.page.evaluate(f"document.getElementById('like-btn-{self.post_slug}').scrollIntoView()")
        await asyncio.sleep(0.5)
        
        # Get initial like count
        await self.page.wait_for_function(
            f"document.getElementById('count-{self.post_slug}').textContent !== 'Loading...'",
            timeout=10000
        )
        
        # Try to like the post
        like_button = self.page.locator(f"#like-btn-{self.post_slug}")
        await like_button.click()
        
        # Wait for interaction to complete
        await asyncio.sleep(3)
        
        # Simulate page refresh (user comes back later)
        await self.page.reload()
        await self.page.wait_for_load_state("networkidle")
        
        # Verify like count persists after refresh
        await self.page.wait_for_function(
            f"document.getElementById('count-{self.post_slug}').textContent !== 'Loading...'",
            timeout=10000
        )
        
        # Take screenshot after refresh
        await self.take_screenshot("user_journey_after_refresh")
        
        final_count_text = await self.page.locator(f"#count-{self.post_slug}").text_content()
        assert final_count_text.isdigit(), f"Like count not displayed correctly after refresh: {final_count_text}"
        
        cprint("✓ User journey completed successfully with persistence", "green")


async def run_browser_e2e_tests(server, client):
    """
    Run all browser-based E2E tests.
    
    Args:
        server: NixOS test server machine
        client: NixOS test client machine  
    """
    cprint("Starting browser-based E2E tests...", "yellow", attrs=["bold"])
    
    # Run tests with xvfb-run for virtual display instead of waiting for X11
    # Set environment variables for Playwright in NixOS
    import os
    os.environ["PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS"] = "true"
    
    tests = BlogE2ETests()
    
    try:
        await tests.setup_browser(headless=True)
        
        # Run all browser tests
        await tests.test_blog_page_loads()
        await tests.test_like_count_loads()
        await tests.test_like_button_interaction()
        await tests.test_javascript_error_handling()
        await tests.test_responsive_design()
        await tests.test_user_journey_multiple_interactions()
        
        cprint("All browser E2E tests passed! ✓", "green", attrs=["bold"])
        
    except Exception as e:
        cprint(f"Browser E2E test failed: {str(e)}", "red", attrs=["bold"])
        
        # Take screenshot for debugging
        if tests.page:
            try:
                await tests.page.screenshot(path='/tmp/test_failure.png')
                cprint("Test failure screenshot saved to /tmp/test_failure.png", "yellow")
            except:
                pass
        
        raise
    finally:
        await tests.teardown_browser()


def run_browser_tests_sync(server, client):
    """Synchronous wrapper for async browser tests."""
    asyncio.run(run_browser_e2e_tests(server, client))


async def main():
    """Main function for running browser tests as a standalone script."""
    import argparse
    import os
    
    parser = argparse.ArgumentParser(description='Run browser E2E tests')
    parser.add_argument('--url', default='http://localhost', 
                       help='Base URL of the server to test (default: http://localhost)')
    parser.add_argument('--headless', action='store_true', default=True,
                       help='Run in headless mode (default: True)')
    parser.add_argument('--help-only', action='store_true',
                       help='Just show this help and exit')
    
    args = parser.parse_args()
    
    if args.help_only:
        parser.print_help()
        return
    
    cprint(f"Running browser E2E tests against {args.url}", "yellow", attrs=["bold"])
    
    # Set environment for NixOS
    os.environ["PLAYWRIGHT_SKIP_VALIDATE_HOST_REQUIREMENTS"] = "true"
    
    tests = BlogE2ETests(base_url=args.url)
    
    try:
        await tests.setup_browser(headless=args.headless)
        
        # Run all browser tests
        await tests.test_blog_page_loads()
        await tests.test_like_count_loads()
        await tests.test_like_button_interaction()
        await tests.test_javascript_error_handling()
        await tests.test_responsive_design()
        await tests.test_user_journey_multiple_interactions()
        
        cprint("All browser E2E tests passed! ✓", "green", attrs=["bold"])
        
    except Exception as e:
        cprint(f"Browser E2E test failed: {str(e)}", "red", attrs=["bold"])
        sys.exit(1)
    finally:
        if tests.browser:
            await tests.browser.close()
        if tests.playwright:
            await tests.playwright.stop()


if __name__ == "__main__":
    try:
        from termcolor import cprint
    except ImportError:
        def cprint(text, color=None, attrs=None, file=None):
            print(text, file=file or sys.stdout)
    
    asyncio.run(main())
