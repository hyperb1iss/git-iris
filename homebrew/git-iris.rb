# typed: false
# frozen_string_literal: true

# Homebrew formula for Git-Iris
# To use: brew tap hyperb1iss/tap && brew install git-iris
class GitIris < Formula
  desc "AI-powered Git workflow assistant with intelligent commit messages"
  homepage "https://github.com/hyperb1iss/git-iris"
  license "Apache-2.0"
  version "2.0.0"

  on_macos do
    on_arm do
      url "https://github.com/hyperb1iss/git-iris/releases/download/v#{version}/git-iris-macos-arm64"
      sha256 "PLACEHOLDER_MACOS_ARM64_SHA256"
    end
    on_intel do
      # No pre-built Intel binary; build from source
      url "https://github.com/hyperb1iss/git-iris/archive/refs/tags/v#{version}.tar.gz"
      sha256 "PLACEHOLDER_SOURCE_SHA256"
      depends_on "rust" => :build
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/hyperb1iss/git-iris/releases/download/v#{version}/git-iris-linux-arm64"
      sha256 "PLACEHOLDER_LINUX_ARM64_SHA256"
    end
    on_intel do
      url "https://github.com/hyperb1iss/git-iris/releases/download/v#{version}/git-iris-linux-amd64"
      sha256 "PLACEHOLDER_LINUX_AMD64_SHA256"
    end
  end

  def install
    if OS.mac? && Hardware::CPU.intel?
      # Building from source (Intel Mac)
      system "cargo", "install", *std_cargo_args
    else
      # Pre-built binary
      bin.install "git-iris-macos-arm64" => "git-iris" if OS.mac? && Hardware::CPU.arm?
      bin.install "git-iris-linux-arm64" => "git-iris" if OS.linux? && Hardware::CPU.arm?
      bin.install "git-iris-linux-amd64" => "git-iris" if OS.linux? && Hardware::CPU.intel?
    end
  end

  test do
    assert_match "git-iris #{version}", shell_output("#{bin}/git-iris --version")
  end
end
