# Legal Notices, Trademark Policy & DMCA Compliance

This document outlines the legal compliance, trademark policies, originality standards, and copyright takedown procedures for the **`nanti.`** project.

---

## 1. Project License

`nanti.` is free and open-source software licensed under the [MIT License](LICENSE). You are free to use, modify, distribute, and sublicense the software in accordance with the terms of the MIT License.

---

## 2. Trademark Notices & Nominative Fair Use

- **"Visual Studio Code" and "VS Code"** are registered trademarks of **Microsoft Corporation**.
- **"Zed"** is a trademark of **Zed Industries, Inc.**
- **"Claude" and "Claude Code"** are trademarks of **Anthropic PBC**.
- **"OpenAI" and "Codex"** are trademarks of **OpenAI, Inc.**
- **"Windows"** is a registered trademark of **Microsoft Corporation**.

### Nominative Fair Use Statement
`nanti.` is an independent, clean-room open-source project and is **not affiliated with, sponsored by, endorsed by, or associated with Microsoft Corporation, Anthropic PBC, OpenAI, Inc., Zed Industries, Inc., or any of their affiliates.**

Any references to "Visual Studio Code", "VS Code", "VS Code Extension API", or related terms within this repository, binary releases, documentation, or source code (including variable names and file paths such as `vscode-api.mjs`) are made **strictly for identification and factual technical compatibility purposes** under the legal doctrine of **Nominative Fair Use**. Such references do not imply endorsement, sponsorship, or association.

`nanti.` does not use any Microsoft trademarks, logos, proprietary icon sets (such as Microsoft Codicons), or visual identities. All logos, icons, and visual branding in `nanti.` are either original works or licensed under permissive open-source licenses.

---

## 3. Marketplace Policy & Ecosystem Compliance

### No Access to Microsoft Visual Studio Marketplace
Under the Microsoft Visual Studio Marketplace Terms of Use, access to the Visual Studio Marketplace is restricted to official Microsoft products and services. To maintain strict legal and regulatory compliance:

1. **`nanti.` does NOT connect to, query, download from, or scrape `marketplace.visualstudio.com`.**
2. `nanti.` does not provide any feature, setting, or background service that redirects queries to Microsoft's proprietary marketplace.
3. For public extension discovery and installation, `nanti.` exclusively integrates with the open-access **Open VSX Registry** (`open-vsx.org`), hosted and managed by the **Eclipse Foundation**.

### Third-Party Extensions & User Responsibility
`nanti.` does not bundle, host, mirror, or redistribute proprietary third-party extension packages (including, but not limited to, `.vsix` packages from Claude Code, Codex, or GitHub Copilot). 

Users who choose to install local `.vsix` packages or download extensions from third-party sources do so independently, at their own discretion, and remain solely responsible for reviewing and complying with the respective extension vendor's license agreements and terms of service.

---

## 4. Privacy, Zero-Telemetry & Local-First Commitment

`nanti.` is architected on the premise of uncompromising developer privacy and local-first execution:

- **Strict Zero-Telemetry Policy:** `nanti.` contains **no telemetry collection code, no analytics beacons, no crash reporting daemons, and no unique machine fingerprinting**. The core executable operates completely offline and never initiates background network pings.
- **Local-First Architecture:** All code editing, tree-sitter parsing, syntax highlighting, Git operations, and terminal shell commands run entirely on the local device. Your intellectual property and code never leave your machine.
- **Explicit Consent Isolation:** If an extension attempts to open an external web resource, `nanti.` halts the action and presents an explicit confirmation dialogue to the user before launching the system browser.
- **Privacy by Design:** By storing zero personal data and avoiding all remote telemetry transmissions, `nanti.` inherently satisfies strict enterprise privacy frameworks and global data protection regulations (including GDPR and CCPA).

---

## 5. Clean-Room Implementation & Originality

- **Clean-Room Engineering:** All source code in `nanti.` (including the workbench shell, rope buffer editor, terminal emulator interface, LSP client, Git wrapper, and Node.js extension host shim) has been written independently from scratch in Rust and JavaScript.
- **Interoperability Under Copyright Law:** The extension API shim implements compatibility based solely on publicly documented APIs, public TypeScript type definitions, and observed runtime behavior. This implementation is protected under international copyright interoperability provisions, including:
  - United States Copyright Act (17 U.S.C. § 1201(f) - Reverse Engineering and Interoperability).
  - European Union Directive 2009/24/EC on the legal protection of computer programs (Articles 5 and 6 regarding decompilation and interoperability).
- **No Proprietary Code Incorporation:** No decompiled, reverse-engineered proprietary code, or vendor-locked intellectual property has been incorporated into `nanti.`.

---

## 5. Third-Party Open Source Compliance

`nanti.` builds upon high-quality open-source libraries (including GPUI, alacritty_terminal, tree-sitter, ropey, wry, and others). All third-party components are used under permissive licenses (MIT, Apache-2.0, BSD, ISC, Zlib, MPL-2.0).

- Complete dependency listings and license classifications are documented in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
- Full verbatim license texts are compiled in [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt).
- All original copyright notices and disclaimers are preserved in accordance with each license's requirements.

---

## 6. DMCA & Copyright Infringement Takedown Policy

The `nanti.` project respects the intellectual property rights of others and complies with the **Digital Millennium Copyright Act (17 U.S.C. § 512)**.

If you are a copyright owner, trademark owner, or authorized agent thereof, and believe that any content, code, or material hosted in this repository infringes upon your intellectual property rights, please submit a formal notification before pursuing platform-level takedowns or legal action:

### Notification Requirements
Your written notice must include:
1. Identification of the copyrighted work or trademark claimed to have been infringed.
2. Identification of the material that is claimed to be infringing (including repository file paths or commit hashes).
3. Your contact information (name, address, telephone number, and email address).
4. A statement that you have a good-faith belief that use of the material in the manner complained of is not authorized by the copyright owner, its agent, or the law.
5. A statement, made under penalty of perjury, that the information in your notification is accurate, and that you are authorized to act on behalf of the owner of an exclusive right that is allegedly infringed.
6. A physical or electronic signature of the authorized person.

### Designated Contact for Notices
- **Email:** `legal@nanti.dev`
- **Repository Issues:** For informal inquiries or clarification, please open an issue labeled `legal` or `compliance` in this repository.

Upon receipt of a valid notice, the maintainers will investigate promptly and take appropriate corrective action (including modification or removal of the disputed material).

---

## 7. Disclaimer of Warranties & Limitation of Liability

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE, TITLE, AND NONINFRINGEMENT. 

IN NO EVENT SHALL THE AUTHORS, COPYRIGHT HOLDERS, OR CONTRIBUTORS BE LIABLE FOR ANY CLAIM, DAMAGES, OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT, OR OTHERWISE, ARISING FROM, OUT OF, OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE, INCLUDING BUT NOT LIMITED TO DATA LOSS, SYSTEM DAMAGE, INTERRUPTION OF SERVICE, OR THIRD-PARTY EXTENSION BEHAVIOR.

