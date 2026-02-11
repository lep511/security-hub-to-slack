# AWS SCP Generator

An interactive Service Control Policies (SCPs) generator for AWS Organizations, based on examples from the official AWS repository.

## 🎯 Features

✅ **Automatic loading** of SCP templates from JSON files
🎨 **Interactive interface** with selection menus
📂 **Organization by categories** (security, compliance, security-services, etc.)
🔍 **Search** for policies by name or description
🚀 **Automatic deployment** to AWS Organizations
📎 **Flexible attachment** to OUs, accounts, or the root
📋 **Visualization** of currently deployed SCPs

## 📦 Installation

### Prerequisites

* **Rust 1.70+** installed
* **AWS CLI** configured with credentials
* Access to **AWS Organizations** (management account)

### Build the project

```bash
git clone <your-repo>
cd scp-generator
cargo build --release

```

The binary will be located at `target/release/scp-generator`.

## 🚀 Usage

### 1. Initialize example templates

```bash
cargo run -- --init

```

This creates an `scp-templates/` directory with example policies organized by category.

### 2. Interactive Mode (Main)

```bash
cargo run

```

Or, if you have already built the release version:

```bash
./target/release/scp-generator

```

### 3. List available templates only

```bash
cargo run -- --list-only

```

### 4. Use a custom template directory

```bash
cargo run -- --templates-dir /path/to/my/templates

```

## 📁 Template Structure

Templates must be JSON files following the standard SCP format:

```text
scp-templates/
├── security/
│   ├── deny-root-user-access.json
│   └── deny-iam-changes-except-sso.json
├── security-services/
│   ├── deny-guardduty-changes.json
│   ├── deny-cloudtrail-changes.json
│   └── deny-config-changes.json
└── compliance/
    └── deny-region-restriction.json

```

**JSON File Example:**

```json
{
    "Version": "2012-10-17",
    "Statement": [
        {
            "Sid": "ReadableDescription",
            "Effect": "Deny",
            "Action": ["service:Action"],
            "Resource": "*"
        }
    ]
}

```

## 🔧 AWS Configuration

Ensure your AWS credentials are configured:

**Option 1: Environment Variables**

```bash
export AWS_ACCESS_KEY_ID=your_access_key
export AWS_SECRET_ACCESS_KEY=your_secret_key
export AWS_REGION=us-east-1

```

**Option 2: AWS CLI (Recommended)**

```bash
aws configure

```

**Required Permissions:**

* `organizations:CreatePolicy`
* `organizations:AttachPolicy`
* `organizations:ListPolicies`
* `organizations:ListRoots`
* `organizations:ListOrganizationalUnitsForParent`

## 🎮 Workflow

1. Run the program: `cargo run`
2. Navigate the main menu and choose an option.
3. Select an SCP by category or via search.
4. Review the policy details.
5. Customize the name and description if desired.
6. Create the SCP in AWS Organizations.
7. Optionally attach the policy to an OU or account.

## 📖 Included SCP Examples

### Security

* **deny-root-user-access:** Blocks direct use of root credentials.
* **deny-iam-changes-except-sso:** Centralizes IAM management in AWS SSO.

### Security Services

* **deny-guardduty-changes:** Protects GuardDuty configuration.
* **deny-cloudtrail-changes:** Prevents modification of trails.
* **deny-config-changes:** Protects AWS Config.

### Compliance

* **deny-region-restriction:** Restricts usage to approved regions.

## 🛠️ Development

### Adding new SCPs

1. Create a JSON file in the appropriate directory.
2. Use the standard SCP format.
3. The program will detect it automatically.

### Code Structure

* `src/`
* `main.rs`: Main application and flow logic.
* `models.rs`: Data structures (ScpPolicy, Statement, etc.).
* `loader.rs`: Loading templates from files.
* `aws.rs`: Interaction with the AWS Organizations API.
* `ui.rs`: Interactive user interface.



## ⚠️ Important Considerations

* SCPs take effect **immediately** upon attachment.
* Test in development OUs first.
* **DO NOT** attach restrictive SCPs to the management account without caution.
* Document all policies you deploy.
* Maintain a "break-glass" process for emergencies.

## 🐛 Troubleshooting

**Error: "Could not find credentials"**

```bash
# Configure AWS CLI
aws configure

```

**Error: "Access Denied" when creating SCP**
Verify that your user/role has Organizations Admin permissions.

**No templates appear**

```bash
# Check the path
cargo run -- --templates-dir ./scp-templates

# Or initialize examples
cargo run -- --init

```

## 📝 License

MIT

## 🤝 Contributions

Contributions are welcome! Please:

1. Fork the repository.
2. Create a branch for your feature.
3. Commit your changes.
4. Push and create a Pull Request.

## 🔗 References

* [AWS Service Control Policies Documentation](https://docs.aws.amazon.com/organizations/latest/userguide/orgs_manage_policies_scps.html)
* [AWS SCP Examples Repository](https://github.com/aws-samples/service-control-policy-examples)
* [AWS Organizations Best Practices](https://docs.aws.amazon.com/organizations/latest/userguide/orgs_best-practices.html)
