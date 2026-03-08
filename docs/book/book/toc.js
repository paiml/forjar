// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="index.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li class="chapter-item expanded "><a href="01-getting-started.html"><strong aria-hidden="true">2.</strong> Getting Started</a></li><li class="chapter-item expanded "><a href="02-configuration.html"><strong aria-hidden="true">3.</strong> Configuration Reference</a></li><li class="chapter-item expanded "><a href="03-resources.html"><strong aria-hidden="true">4.</strong> Resource Types</a></li><li class="chapter-item expanded "><a href="04-recipes.html"><strong aria-hidden="true">5.</strong> Recipes</a></li><li class="chapter-item expanded "><a href="05-architecture.html"><strong aria-hidden="true">6.</strong> Architecture</a></li><li class="chapter-item expanded "><a href="06-cli.html"><strong aria-hidden="true">7.</strong> CLI Reference</a></li><li class="chapter-item expanded "><a href="07-cookbook.html"><strong aria-hidden="true">8.</strong> Cookbook</a></li><li class="chapter-item expanded "><a href="08-state-management.html"><strong aria-hidden="true">9.</strong> State Management</a></li><li class="chapter-item expanded "><a href="09-drift-and-tripwire.html"><strong aria-hidden="true">10.</strong> Drift Detection &amp; Tripwire</a></li><li class="chapter-item expanded "><a href="10-testing-and-ci.html"><strong aria-hidden="true">11.</strong> Testing &amp; CI/CD Integration</a></li><li class="chapter-item expanded "><a href="11-troubleshooting.html"><strong aria-hidden="true">12.</strong> Troubleshooting</a></li><li class="chapter-item expanded "><a href="12-store.html"><strong aria-hidden="true">13.</strong> Content-Addressed Store</a></li><li class="chapter-item expanded "><a href="13-formal-verification.html"><strong aria-hidden="true">14.</strong> Formal Verification &amp; Provability</a></li><li class="chapter-item expanded "><a href="14-state-safety.html"><strong aria-hidden="true">15.</strong> State Safety &amp; Disaster Recovery</a></li><li class="chapter-item expanded "><a href="15-dataops-mlops.html"><strong aria-hidden="true">16.</strong> DataOps &amp; MLOps Pipelines</a></li><li class="chapter-item expanded "><a href="16-agent-infrastructure.html"><strong aria-hidden="true">17.</strong> Agent Infrastructure &amp; pforge</a></li><li class="chapter-item expanded "><a href="17-operational-intelligence.html"><strong aria-hidden="true">18.</strong> Operational Intelligence</a></li><li class="chapter-item expanded "><a href="18-supply-chain-security.html"><strong aria-hidden="true">19.</strong> Supply Chain Security &amp; Resilience</a></li><li class="chapter-item expanded "><a href="19-competitive-positioning.html"><strong aria-hidden="true">20.</strong> Competitive Positioning</a></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString();
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
