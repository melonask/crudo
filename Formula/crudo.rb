class Crudo < Formula
  desc "Configuration-driven JSON APIs backed by SQL"
  homepage "https://github.com/melonask/crudo"
  license "MIT"
  head "https://github.com/melonask/crudo.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", "--locked", "--root", prefix, "--path", "."
  end

  test do
    assert_match "crudo", shell_output("#{bin}/crudo --version")
  end
end
