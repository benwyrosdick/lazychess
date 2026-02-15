class Lazychess < Formula
  desc 'A chess analysis TUI with Stockfish integration'
  homepage 'https://github.com/benwyrosdick/lazychess'
  version '0.1.0'
  license 'GPL-3.0'

  depends_on 'stockfish' => :recommended

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/benwyrosdick/lazychess/releases/download/v#{version}/lazychess-aarch64-apple-darwin.tar.gz"
      sha256 'SHA256_ARM_DARWIN'
    else
      url "https://github.com/benwyrosdick/lazychess/releases/download/v#{version}/lazychess-x86_64-apple-darwin.tar.gz"
      sha256 'SHA256_X86_DARWIN'
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      odie 'lazychess is not currently supported on Linux ARM'
    else
      url "https://github.com/benwyrosdick/lazychess/releases/download/v#{version}/lazychess-x86_64-unknown-linux-gnu.tar.gz"
      sha256 'SHA256_LINUX'
    end
  end

  def install
    bin.install 'lazychess'
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/lazychess --version")
  end
end
